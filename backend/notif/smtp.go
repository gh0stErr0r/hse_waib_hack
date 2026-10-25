package main

import (
	"crypto/tls"
	"errors"
	"fmt"
	"io"
	"mime"
	"net"
	"net/mail"
	"net/smtp"
	"os"
	"strconv"
	"strings"
	"time"
)

type smtpSettings struct {
	host, port, username, password, network, security string
	timeout                                           time.Duration
	tlsConfig                                         *tls.Config
}

func smtpSettingsFromEnv(provider string) (smtpSettings, error) {
	var host string
	switch provider {
	case "gmail":
		host = "smtp.gmail.com"
	case "yandex":
		host = "smtp.yandex.ru"
	default:
		return smtpSettings{}, errors.New("unsupported SMTP provider")
	}
	network := envOrDefault("SMTP_NETWORK", "tcp")
	if network != "tcp" && network != "tcp4" && network != "tcp6" {
		return smtpSettings{}, errors.New("SMTP_NETWORK must be tcp, tcp4 or tcp6")
	}
	timeout, err := time.ParseDuration(envOrDefault("SMTP_TIMEOUT", "20s"))
	if err != nil || timeout < time.Second || timeout > time.Minute {
		return smtpSettings{}, errors.New("SMTP_TIMEOUT must be between 1s and 1m")
	}
	security := envOrDefault("SMTP_SECURITY", "starttls")
	if security != "starttls" && security != "tls" {
		return smtpSettings{}, errors.New("SMTP_SECURITY must be starttls or tls")
	}
	defaultPort := "587"
	if security == "tls" {
		defaultPort = "465"
	}
	port := envOrDefault("SMTP_PORT", defaultPort)
	number, err := strconv.Atoi(port)
	if err != nil || number < 1 || number > 65535 {
		return smtpSettings{}, errors.New("SMTP_PORT must be between 1 and 65535")
	}
	host = envOrDefault("SMTP_HOST", host)
	return smtpSettings{host: host, port: port, username: strings.TrimSpace(os.Getenv("SMTP_USER")), password: os.Getenv("SMTP_PASSWORD"), network: network, security: security, timeout: timeout}, nil
}

func openSMTP(settings smtpSettings) (*smtp.Client, error) {
	if settings.username == "" || settings.password == "" {
		return nil, errors.New("SMTP credentials are missing")
	}
	address := net.JoinHostPort(settings.host, settings.port)
	conn, err := net.DialTimeout(settings.network, address, settings.timeout)
	if err != nil {
		return nil, fmt.Errorf("SMTP connect (%s): %w", settings.network, err)
	}
	_ = conn.SetDeadline(time.Now().Add(settings.timeout))
	config := settings.tlsConfig
	if config == nil {
		config = &tls.Config{ServerName: settings.host, MinVersion: tls.VersionTLS12}
	}
	if settings.security == "tls" {
		secured := tls.Client(conn, config)
		if err = secured.Handshake(); err != nil {
			_ = conn.Close()
			return nil, fmt.Errorf("SMTP TLS: %w", err)
		}
		conn = secured
	}
	client, err := smtp.NewClient(conn, settings.host)
	if err != nil {
		_ = conn.Close()
		return nil, fmt.Errorf("SMTP greeting: %w", err)
	}
	fail := func(stage string, err error) (*smtp.Client, error) {
		_ = client.Close()
		return nil, fmt.Errorf("SMTP %s: %w", stage, err)
	}
	if err = client.Hello("localhost"); err != nil {
		return fail("EHLO", err)
	}
	if settings.security != "tls" {
		if ok, _ := client.Extension("STARTTLS"); !ok {
			return fail("STARTTLS", errors.New("server does not support required TLS"))
		}
		if err = client.StartTLS(config); err != nil {
			return fail("STARTTLS", err)
		}
	}

	if err = client.Auth(smtp.PlainAuth("", settings.username, settings.password, settings.host)); err != nil {
		return fail("authentication", err)
	}
	return client, nil
}

func sendSMTP(settings smtpSettings, recipient, subject, body string) error {
	to, err := mail.ParseAddress(recipient)
	if err != nil {
		return fmt.Errorf("invalid recipient: %w", err)
	}
	from, err := mail.ParseAddress(settings.username)
	if err != nil {
		return fmt.Errorf("invalid sender: %w", err)
	}
	if strings.ContainsAny(subject, "\r\n") {
		return errors.New("invalid subject")
	}
	client, err := openSMTP(settings)
	if err != nil {
		return err
	}
	defer client.Close()
	if err = client.Mail(from.Address); err != nil {
		return fmt.Errorf("SMTP MAIL FROM: %w", err)
	}
	if err = client.Rcpt(to.Address); err != nil {
		return fmt.Errorf("SMTP RCPT TO: %w", err)
	}
	writer, err := client.Data()
	if err != nil {
		return fmt.Errorf("SMTP DATA: %w", err)
	}
	normalized := strings.ReplaceAll(strings.ReplaceAll(body, "\r\n", "\n"), "\n", "\r\n")
	message := "To: " + to.String() + "\r\nFrom: " + from.String() + "\r\nSubject: " + mime.QEncoding.Encode("UTF-8", subject) + "\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n" + normalized + "\r\n"
	if _, err = io.WriteString(writer, message); err != nil {
		return fmt.Errorf("SMTP write: %w", err)
	}
	// Close waits for the server's final acceptance. QUIT failure after this is not a delivery failure.
	if err = writer.Close(); err != nil {
		return fmt.Errorf("SMTP acceptance: %w", err)
	}
	_ = client.Quit()
	return nil
}

func checkSMTP() error {
	provider, err := SMTPProviderForEmail(os.Getenv("SMTP_USER"))
	if err != nil {
		return err
	}
	settings, err := smtpSettingsFromEnv(provider)
	if err != nil {
		return err
	}
	client, err := openSMTP(settings)
	if err != nil {
		return err
	}
	defer client.Close()
	_ = client.Quit()
	fmt.Printf("SMTP verified TLS and authentication: OK (%s:%s, %s, %s). No email sent.\n", settings.host, settings.port, settings.network, envOrDefault("SMTP_SECURITY", "starttls"))
	return nil
}
