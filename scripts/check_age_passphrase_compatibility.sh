#!/usr/bin/env bash
# Synthetic interoperability contract for the pre-existing AutoTrace age 0.12.1
# scrypt/passphrase backup format and the audited 0.11.5 dependency.
# No customer files, real keys, tokens or private artifacts are used.
set -euo pipefail
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp"/{v011,v012}/src
for entry in "v011 0.11.5" "v012 0.12.1"; do
  read -r label version <<<"$entry"
  cat > "$tmp/$label/Cargo.toml" <<EOF
[package]
name = "age_format_probe_$label"
version = "0.1.0"
edition = "2021"
[dependencies]
age = "=$version"
EOF
  cat > "$tmp/$label/src/main.rs" <<'RS'
use age::secrecy::SecretString;
use std::{env, fs::File, io::{Read, Write}};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut a = env::args().skip(1);
    let mode = a.next().ok_or("mode")?;
    let path = a.next().ok_or("path")?;
    let password = SecretString::from("AUTOTRACE-SYNTHETIC-NOT-A-REAL-RECOVERY-KEY-2026".to_string());
    if mode == "encrypt" {
        let mut f = age::Encryptor::with_user_passphrase(password).wrap_output(File::create(path)?)?;
        f.write_all(b"AUTOTRACE-DRIVE-1\\nTEST-ONLY-SYNTHETIC-PAYLOAD\\n")?;
        f.finish()?;
    } else if mode == "decrypt" {
        let d = age::Decryptor::new(std::io::BufReader::new(File::open(path)?))?;
        let identity = age::scrypt::Identity::new(password);
        let mut plain = Vec::new();
        d.decrypt(std::iter::once(&identity as &dyn age::Identity))?.read_to_end(&mut plain)?;
        if plain != b"AUTOTRACE-DRIVE-1\\nTEST-ONLY-SYNTHETIC-PAYLOAD\\n" { return Err("plaintext mismatch".into()); }
    } else { return Err("unknown mode".into()); }
    Ok(())
}
RS
done
export CARGO_TARGET_DIR="$tmp/target"
cargo run --quiet --manifest-path "$tmp/v012/Cargo.toml" -- encrypt "$tmp/encrypted-by-v012.age"
cargo run --quiet --manifest-path "$tmp/v011/Cargo.toml" -- decrypt "$tmp/encrypted-by-v012.age"
cargo run --quiet --manifest-path "$tmp/v011/Cargo.toml" -- encrypt "$tmp/encrypted-by-v011.age"
cargo run --quiet --manifest-path "$tmp/v012/Cargo.toml" -- decrypt "$tmp/encrypted-by-v011.age"
# Authenticated ciphertext must fail after mutation: ensure failure never counts as successful recovery.
cp "$tmp/encrypted-by-v012.age" "$tmp/tampered.age"
python3 - "$tmp/tampered.age" <<'PY'
import pathlib,sys
p=pathlib.Path(sys.argv[1]); b=bytearray(p.read_bytes()); b[-3]^=0x55; p.write_bytes(b)
PY
if cargo run --quiet --manifest-path "$tmp/v011/Cargo.toml" -- decrypt "$tmp/tampered.age"; then
  echo "FAIL: modified age ciphertext accepted"; exit 1
fi
echo "PASS: age passphrase format interoperable 0.12.1 <-> 0.11.5; modified ciphertext rejected"
