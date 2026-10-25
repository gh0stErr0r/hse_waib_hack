package crypto

import (
	"errors"
	"strings"
	"testing"

	"golang.org/x/crypto/bcrypt"

	"auth/internal/types"
)

func TestPasswordCompatibility(t *testing.T) {
	service, err := New("test-pepper", "", "")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(service.Close)
	salt := "legacy-salt"
	encoded, err := bcrypt.GenerateFromPassword([]byte("correct-password"+salt+service.pepper), bcrypt.DefaultCost)
	if err != nil {
		t.Fatal(err)
	}
	hash := string(encoded)
	if err := service.CheckPassword("correct-password", salt, hash); err != nil {
		t.Fatal(err)
	}
	if !errors.Is(service.CheckPassword("wrong-password", salt, hash), types.NotEqualPassword) {
		t.Fatal("wrong password accepted")
	}
}

func TestLongPasswordAndPepper(t *testing.T) {
	service, err := New(strings.Repeat("p", 64), "", "")
	if err != nil {
		t.Fatal(err)
	}
	defer service.Close()
	password := strings.Repeat("длинный-пароль", 20)
	hash, salt, err := service.GenerationPassword(password)
	if err != nil {
		t.Fatal(err)
	}
	if !strings.HasPrefix(hash, passwordHashPrefix) {
		t.Fatal("missing versioned password format")
	}
	if err := service.CheckPassword(password, salt, hash); err != nil {
		t.Fatal(err)
	}
	if err := service.CheckPassword(password+"wrong-tail", salt, hash); err == nil {
		t.Fatal("password tail was ignored")
	}
}

func TestServiceSecretsAreIsolated(t *testing.T) {
	first, err := New("first-pepper", "", "")
	if err != nil {
		t.Fatal(err)
	}
	defer first.Close()
	second, err := New("second-pepper", "", "")
	if err != nil {
		t.Fatal(err)
	}
	defer second.Close()
	hash, salt, err := first.GenerationPassword("password")
	if err != nil {
		t.Fatal(err)
	}
	if second.CheckPassword("password", salt, hash) == nil {
		t.Fatal("different service accepted password with another pepper")
	}
	if first.CheckPassword("password", salt, hash) != nil {
		t.Fatal("constructing another service replaced the first pepper")
	}
	if string(first.PublicKey()) == string(second.PublicKey()) {
		t.Fatal("service keys shared")
	}
}
