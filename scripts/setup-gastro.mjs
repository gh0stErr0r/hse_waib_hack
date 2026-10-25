#!/usr/bin/env node
import {
  generateKeyPairSync,
  randomBytes,
  createPrivateKey,
  createPublicKey,
} from "node:crypto";
import { readFileSync, writeFileSync, existsSync, chmodSync } from "node:fs";
import { fileURLToPath } from "node:url";
const destination = fileURLToPath(new URL("../infra/.env", import.meta.url));
const template = fileURLToPath(
  new URL("../infra/.env.example", import.meta.url),
);
let content = readFileSync(
  existsSync(destination) ? destination : template,
  "utf8",
);
const get = (key) =>
  content.match(new RegExp(`^${key}=(.*)$`, "m"))?.[1]?.trim() || "";
const set = (key, value) => {
  const pattern = new RegExp(`^${key}=.*$`, "m");
  content = pattern.test(content)
    ? content.replace(pattern, `${key}=${value}`)
    : `${content}\n${key}=${value}\n`;
};
if (
  !get("POSTGRES_PASSWORD") ||
  get("POSTGRES_PASSWORD") === "replace-with-a-long-random-password"
)
  set("POSTGRES_PASSWORD", randomBytes(24).toString("hex"));
if (!get("PEPPER")) set("PEPPER", randomBytes(32).toString("hex"));
const pub = get("JWT_PUBLIC_KEY"),
  priv = get("JWT_PRIVATE_KEY");
if (Boolean(pub) !== Boolean(priv))
  throw new Error(
    "Оба JWT-ключа должны быть заданы вместе; существующий ключ не заменён",
  );
if (!pub) {
  const { publicKey, privateKey } = generateKeyPairSync("ed25519");
  const publicJwk = publicKey.export({ format: "jwk" }),
    privateJwk = privateKey.export({ format: "jwk" });
  const rawPub = Buffer.from(publicJwk.x, "base64url"),
    seed = Buffer.from(privateJwk.d, "base64url");
  set("JWT_PUBLIC_KEY", rawPub.toString("base64"));
  set("JWT_PRIVATE_KEY", Buffer.concat([seed, rawPub]).toString("base64"));
} else {
  const rawPub = Buffer.from(pub, "base64"),
    rawPriv = Buffer.from(priv, "base64");
  if (rawPub.length !== 32 || rawPriv.length !== 64)
    throw new Error("Неверная длина Ed25519 ключей");
  const key = createPrivateKey({
    key: {
      kty: "OKP",
      crv: "Ed25519",
      x: rawPub.toString("base64url"),
      d: rawPriv.subarray(0, 32).toString("base64url"),
    },
    format: "jwk",
  });
  const actual = createPublicKey(key).export({ format: "jwk" });
  if (
    !Buffer.from(actual.x, "base64url").equals(rawPub) ||
    !rawPriv.subarray(32).equals(rawPub)
  )
    throw new Error("JWT-ключи не совпадают");
}
writeFileSync(destination, content, { mode: 0o600 });
chmodSync(destination, 0o600);
console.log(
  "Файл infra/.env готов: постоянные Ed25519 ключи, pepper и пароль БД настроены. Существующие значения сохранены. Выберите Gemini или локальную Ollama; SMTP необязателен. Секреты не выводятся.",
);
