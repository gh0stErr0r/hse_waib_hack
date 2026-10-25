package main

import (
	"bufio"
	"crypto/tls"
	"crypto/x509"
	"fmt"
	"net"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

func localSMTP(t *testing.T) (smtpSettings, <-chan string, <-chan error) {
	return localSMTPMode(t, false)
}

func localSMTPMode(t *testing.T, implicitTLS bool) (smtpSettings, <-chan string, <-chan error) {
	t.Helper()
	sample := httptest.NewTLSServer(nil)
	certificate := sample.TLS.Certificates[0]
	roots := x509.NewCertPool()
	roots.AddCert(sample.Certificate())
	sample.Close()
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	messages := make(chan string, 1)
	done := make(chan error, 1)
	go func() {
		conn, err := listener.Accept()
		if err != nil {
			done <- err
			return
		}
		defer conn.Close()
		_ = conn.SetDeadline(time.Now().Add(3 * time.Second))
		if implicitTLS {
			secured := tls.Server(conn, &tls.Config{Certificates: []tls.Certificate{certificate}, MinVersion: tls.VersionTLS12})
			if err = secured.Handshake(); err != nil {
				done <- err
				return
			}
			conn = secured
		}
		reader := bufio.NewReader(conn)
		if _, err = fmt.Fprint(conn, "220 localhost SMTP ready\r\n"); err != nil {
			done <- err
			return
		}
		for {
			line, readErr := reader.ReadString('\n')
			if readErr != nil {
				done <- readErr
				return
			}
			switch {
			case strings.HasPrefix(line, "EHLO "):
				_, err = fmt.Fprint(conn, "250-localhost\r\n250-STARTTLS\r\n250 AUTH PLAIN\r\n")
			case line == "STARTTLS\r\n":
				_, err = fmt.Fprint(conn, "220 start TLS\r\n")
				if err != nil {
					done <- err
					return
				}
				secured := tls.Server(conn, &tls.Config{Certificates: []tls.Certificate{certificate}, MinVersion: tls.VersionTLS12})
				if err = secured.Handshake(); err != nil {
					done <- err
					return
				}
				conn = secured
				reader = bufio.NewReader(conn)
			case strings.HasPrefix(line, "AUTH PLAIN "):
				_, err = fmt.Fprint(conn, "235 authenticated\r\n")
			case strings.HasPrefix(line, "MAIL FROM:"), strings.HasPrefix(line, "RCPT TO:"):
				_, err = fmt.Fprint(conn, "250 accepted\r\n")
			case line == "DATA\r\n":
				_, err = fmt.Fprint(conn, "354 send message\r\n")
				if err != nil {
					done <- err
					return
				}
				var body strings.Builder
				for {
					part, readErr := reader.ReadString('\n')
					if readErr != nil {
						done <- readErr
						return
					}
					if part == ".\r\n" {
						break
					}
					body.WriteString(part)
				}
				messages <- body.String()
				_, err = fmt.Fprint(conn, "250 queued\r\n")
				// Deliberately disconnect before QUIT: acceptance must remain a success.
				done <- err
				return
			case line == "QUIT\r\n":
				_, err = fmt.Fprint(conn, "221 bye\r\n")
				done <- err
				return
			default:
				done <- fmt.Errorf("unexpected SMTP command")
				return
			}
			if err != nil {
				done <- err
				return
			}
		}
	}()
	t.Cleanup(func() { _ = listener.Close() })
	host, port, _ := net.SplitHostPort(listener.Addr().String())
	security := "starttls"
	if implicitTLS {
		security = "tls"
	}
	return smtpSettings{security: security, host: host, port: port, username: "sender@example.test", password: "test-password", network: "tcp4", timeout: 2 * time.Second, tlsConfig: &tls.Config{ServerName: host, RootCAs: roots, MinVersion: tls.VersionTLS12}}, messages, done
}

func TestSMTPAcceptsMessageDespiteQuitEOF(t *testing.T) {
	settings, messages, done := localSMTP(t)
	if err := sendSMTP(settings, "recipient@example.test", "Код подтверждения", "Тестовое сообщение\nВторая строка"); err != nil {
		t.Fatal(err)
	}
	select {
	case body := <-messages:
		if !strings.Contains(body, "Subject: =?UTF-8?q?") {
			t.Fatal("subject is not MIME encoded")
		}
		if !strings.Contains(body, "Тестовое сообщение\r\nВторая строка") {
			t.Fatal("missing normalized body")
		}
	case <-time.After(3 * time.Second):
		t.Fatal("message not received")
	}
	if err := <-done; err != nil {
		t.Fatal(err)
	}
}

func TestSMTPRejectsUntrustedTLS(t *testing.T) {
	settings, _, done := localSMTP(t)
	settings.tlsConfig = nil
	if _, err := openSMTP(settings); err == nil || !strings.Contains(err.Error(), "STARTTLS") {
		t.Fatalf("untrusted certificate accepted: %v", err)
	}
	<-done
}

func TestSMTPGreetingTimeout(t *testing.T) {
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatal(err)
	}
	defer listener.Close()
	done := make(chan struct{})
	go func() {
		defer close(done)
		conn, err := listener.Accept()
		if err == nil {
			defer conn.Close()
			_ = conn.SetDeadline(time.Now().Add(time.Second))
			var b [1]byte
			_, _ = conn.Read(b[:])
		}
	}()
	host, port, _ := net.SplitHostPort(listener.Addr().String())
	start := time.Now()
	_, err = openSMTP(smtpSettings{host: host, port: port, network: "tcp4", username: "test", password: "test", timeout: 50 * time.Millisecond})
	if err == nil || !strings.Contains(err.Error(), "greeting") {
		t.Fatalf("missing greeting failure: %v", err)
	}
	if time.Since(start) > time.Second {
		t.Fatal("SMTP deadline not enforced")
	}
	<-done
}

func TestSMTPNetworkConfiguration(t *testing.T) {
	t.Setenv("SMTP_NETWORK", "tcp6")
	t.Setenv("SMTP_TIMEOUT", "15s")
	settings, err := smtpSettingsFromEnv("gmail")
	if err != nil {
		t.Fatal(err)
	}
	if settings.network != "tcp6" || settings.host != "smtp.gmail.com" {
		t.Fatal("SMTP settings not applied")
	}
	t.Setenv("SMTP_NETWORK", "udp")
	if _, err = smtpSettingsFromEnv("gmail"); err == nil {
		t.Fatal("invalid network accepted")
	}
}

func TestSMTPImplicitTLS(t *testing.T) {
	settings, messages, done := localSMTPMode(t, true)
	if err := sendSMTP(settings, "recipient@example.test", "Код подтверждения", "Тест TLS"); err != nil {
		t.Fatal(err)
	}
	select {
	case body := <-messages:
		if !strings.Contains(body, "Тест TLS") {
			t.Fatal("missing message")
		}
	case <-time.After(3 * time.Second):
		t.Fatal("message not received")
	}
	if err := <-done; err != nil {
		t.Fatal(err)
	}
}
func TestSMTPImplicitTLSCertificateVerification(t *testing.T) {
	settings, _, done := localSMTPMode(t, true)
	settings.tlsConfig = nil
	if _, err := openSMTP(settings); err == nil || !strings.Contains(err.Error(), "SMTP TLS") {
		t.Fatalf("untrusted implicit TLS accepted: %v", err)
	}
	<-done
}
func TestSMTPTransportConfiguration(t *testing.T) {
	t.Setenv("SMTP_NETWORK", "tcp")
	t.Setenv("SMTP_SECURITY", "tls")
	t.Setenv("SMTP_PORT", "")
	t.Setenv("SMTP_HOST", "")
	settings, err := smtpSettingsFromEnv("gmail")
	if err != nil {
		t.Fatal(err)
	}
	if settings.port != "465" || settings.security != "tls" {
		t.Fatal("implicit TLS defaults not applied")
	}
	t.Setenv("SMTP_PORT", "bad")
	if _, err := smtpSettingsFromEnv("gmail"); err == nil {
		t.Fatal("invalid SMTP port accepted")
	}
	t.Setenv("SMTP_PORT", "587")
	t.Setenv("SMTP_SECURITY", "none")
	if _, err := smtpSettingsFromEnv("gmail"); err == nil {
		t.Fatal("insecure transport accepted")
	}
}
