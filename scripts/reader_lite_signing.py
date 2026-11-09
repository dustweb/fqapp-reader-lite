#!/usr/bin/env python3
"""Restore the signing secret without revealing any part of its value."""
import base64
import binascii
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys


def fail(reason):
    print(f'::error::Signing setup: {reason}.', file=sys.stderr)
    sys.exit(1)


def main():
    raw = os.environ.get('READER_LITE_SIGNING_JSON', '')
    if not raw:
        fail('the repository secret is empty or unavailable')
    try:
        cfg = json.loads(raw)
    except (json.JSONDecodeError, TypeError):
        fail('the repository secret is not valid JSON')
    if not isinstance(cfg, dict) or not all(k in cfg for k in ('password', 'alias', 'keystore')):
        fail('the JSON does not contain the required signing fields')
    password, alias = cfg['password'], cfg['alias']
    if not isinstance(password, str) or not re.fullmatch(r'[A-Za-z0-9_-]{24,}', password):
        fail('the signing password has an invalid format')
    if alias != 'reader-lite':
        fail('the key alias does not match')
    try:
        key_bytes = base64.b64decode(cfg['keystore'], validate=True)
    except (TypeError, ValueError, binascii.Error):
        fail('the signing key encoding is invalid')
    if not key_bytes or len(key_bytes) > 32768:
        fail('the signing key has an invalid size')
    key_path = pathlib.Path(os.environ['RUNNER_TEMP']) / 'reader-lite.p12'
    key_path.write_bytes(key_bytes)
    key_path.chmod(0o600)
    env = dict(os.environ, RELEASE_STORE_PASSWORD=password)
    try:
        cert = subprocess.check_output(
            ['keytool', '-exportcert', '-keystore', str(key_path), '-storepass:env',
             'RELEASE_STORE_PASSWORD', '-alias', alias], env=env, stderr=subprocess.DEVNULL)
    except (OSError, subprocess.CalledProcessError):
        fail('the keystore could not be opened with its password and alias')
    expected = os.environ['EXPECTED_SIGNER_SHA256'].lower()
    if hashlib.sha256(cert).hexdigest() != expected:
        fail('the signing certificate fingerprint does not match')
    print(f'::add-mask::{password}', flush=True)
    with open(os.environ['GITHUB_ENV'], 'a', encoding='utf-8') as output:
        output.write(f'RELEASE_KEYSTORE_FILE={key_path}\n')
        output.write(f'RELEASE_STORE_PASSWORD={password}\n')
        output.write(f'RELEASE_KEY_ALIAS={alias}\n')
        output.write(f'RELEASE_KEY_PASSWORD={password}\n')


if __name__ == '__main__':
    main()
