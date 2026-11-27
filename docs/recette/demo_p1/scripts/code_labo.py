#!/usr/bin/env python3
"""Aide de LABO (base locale uniquement) : retrouve le code de préinscription le plus récent
d'une adresse à partir de son empreinte (clé de développement publique SecretBox::dev())."""
import hashlib, hmac, subprocess, sys
email = sys.argv[1]
key = hashlib.sha256(b"scolia-ent-dev-secrets-key-PUBLIC-NOT-SECRET").digest()
dk = hashlib.sha256(b"ent-secretbox-derive-v1\0" + key + b"\0" + b"preinscription-code-v1").digest()
out = subprocess.check_output(["psql","-h","127.0.0.1","-U","ent_ci","-d","ent_ci_test_a","-Atc",
  f"select id||'|'||code_hash from ent_preinscription_verification where email='{email}' and verifie_le is null order by created_at desc limit 1"],
  env={"PGPASSWORD":"ci","PATH":"/usr/bin:/bin"}).decode().strip()
did, h = out.split("|")
for n in range(1000000):
    c = f"{n:06d}"
    if hmac.new(dk, f"{did}:{c}".encode(), hashlib.sha256).hexdigest() == h:
        print(c); sys.exit(0)
sys.exit(1)
