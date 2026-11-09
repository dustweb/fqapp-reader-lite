#!/usr/bin/env python3
"""Restore one private signing secret without printing its contents."""
import base64
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys


def main():
    try:
        cfg = json.loads(os.environ['READER_LITE_SIGNING_JSON'])
        password = cfg['password']
        alias = cfg['alias']
        if not isinstance(password, str) or not re.fullmatch(r'[A-Za-z0-9_-]{24,}', password):
            raise ValueError('invalid password format')
        if alias != 'reader-lite':
            raise ValueError('invalid alias')
        key_bytes = base64.b64decode(cfg['keystore'], validate=True)
        if not key_bytes or len(key_bytes) > 32768:
            raise ValueError('invalid key size')
        key_path = pathlib.Path(os.environ['RUNNER_TEMP']) / 'reader-lite.p12'
        key_path.write_bytes(key_bytes)
        key_path.chmod(0o600)
        env = dict(os.environ, RELEASE_STORE_PASSWORD=password)
        cert = subprocess.check_output(
            ['keytool', '-exportcert', '-keystore', str(key_path), '-storepass:env',
             'RELEASE_STORE_PASSWORD', '-alias', alias], env=env, stderr=subprocess.DEVNULL)
        expected = os.environ['EXPECTED_SIGNER_SHA256'].lower()
        if hashlib.sha256(cert).hexdigest() != expected:
            raise ValueError('certificate fingerprint does not match')
        print(f'::add-mask::{password}', flush=True)
        with open(os.environ['GITHUB_ENV'], 'a', encoding='utf-8') as output:
            output.write(f'RELEASE_KEYSTORE_FILE={key_path}\n')
            output.write(f'RELEASE_STORE_PASSWORD={password}\n')
            output.write(f'RELEASE_KEY_ALIAS={alias}\n')
            output.write(f'RELEASE_KEY_PASSWORD={password}\n')
    except Exception:
        print('::error::Signing secret is missing, invalid, or uses the wrong certificate.', file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
