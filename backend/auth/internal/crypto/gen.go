package crypto

import (
	"crypto/hmac"
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"runtime/secret"
	"strings"

	"golang.org/x/crypto/bcrypt"

	"auth/internal/types"
)

func generateSalt() string {
	var salt [16]byte
	_, _ = rand.Read(salt[:])
	return base64.StdEncoding.EncodeToString(salt[:])
}

func (s *Service) passwordMaterial(password, salt string) []byte {
	material := make([]byte, 0, len(password)+len(salt)+len(s.pepper))
	material = append(material, password...)
	material = append(material, salt...)
	return append(material, s.pepper...)
}

const passwordHashPrefix = "$gastro-hmac-sha256-v1$"

// Prehash to a fixed, printable length: bcrypt accepts at most 72 bytes.
// Include the stored random salt and a domain separator; use pepper as HMAC key.
func (s *Service) prehashedPasswordMaterial(password, salt string) []byte {
	key := []byte(s.pepper)
	defer clear(key)
	mac := hmac.New(sha256.New, key)
	_, _ = mac.Write([]byte("gastro/password/v1\x00"))
	_, _ = mac.Write([]byte(salt))
	_, _ = mac.Write([]byte{0})
	_, _ = mac.Write([]byte(password))
	digest := mac.Sum(nil)
	defer clear(digest)
	return base64.RawStdEncoding.AppendEncode(nil, digest)
}

func (s *Service) GenerationPassword(password string) (string, string, error) {
	salt := generateSalt()
	var hash string
	var hashErr error
	secret.Do(func() {
		material := s.prehashedPasswordMaterial(password, salt)
		defer clear(material)
		hashed, err := bcrypt.GenerateFromPassword(material, bcrypt.DefaultCost)
		if err != nil {
			hashErr = err
			return
		}
		hash = passwordHashPrefix + string(hashed)
		clear(hashed)
	})
	if hashErr != nil {
		return "", "", hashErr
	}
	return hash, salt, nil
}

func (s *Service) CheckPassword(password, salt, hash string) error {
	var compareErr error
	secret.Do(func() {
		var material []byte
		encoded := hash
		if strings.HasPrefix(hash, passwordHashPrefix) {
			material = s.prehashedPasswordMaterial(password, salt)
			encoded = strings.TrimPrefix(hash, passwordHashPrefix)
		} else {
			// Verify existing accounts without changing their stored hash format.
			material = s.passwordMaterial(password, salt)
		}
		defer clear(material)
		compareErr = bcrypt.CompareHashAndPassword([]byte(encoded), material)
	})
	if compareErr != nil {
		return types.NotEqualPassword
	}
	return nil
}
