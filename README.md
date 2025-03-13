# Tartarus - Rust Locker


## Overview

The Hyperswitch Card Vault (Tartarus) is a highly performant and a secure vault to save sensitive data such as payment card details, bank account details etc.

It is designed in an polymorphic manner to handle and store any type of sensitive information making it highly scalable with extensive coverage of payment methods and processors.

Tartarus is built with a GDPR compliant personal identifiable information (PII) storage and secure encryption algorithms to be fully compliant with PCI DSS requirements.

Here's a quick guide to [Get Started](./docs/guides/setup.md) with setting up Tartarus.

### How does Tartarus work?

- Your application will communicate with Tartarus via a middleware.
- All requests and responses to and from the middleware are signed and encrypted with the JWS and JWE algorithms.
- The locker supports CRD APIs on the /data and /cards endpoints - <API Reference to be linked>
- Cards are stored against the combination of merchant and customer identifiers.
- Internal hashing checks are in place to avoid data duplication.

![General Work Flow](./docs/imgs/general-block-diagram.png)

### Key Hierarchy

- Master Key - AES generated key to that is encrypted/decrypted by the custodian keys to run the locker and associated configurations.
- Custodian Keys - AES generated key that is used to encrypt and decrypt the master key. It broken into two keys (key 1 and key 2) and available with two custodians to enhance security.

![Key Hierarchy](./docs/imgs/locker-key-hierarchy.png)

### Setup Guide

Follow this guide to setup Tartarus - [Get Started](./docs/guides/setup.md)


### T2C Setup guide

1. Generate master-key without key_custodian.

        cargo run --bin utils -- master-key -w > locker_master_key.txt
    
2. Configure AWS KMS and AWS CLI. Also, congifure the AWS KMS key_id and region in vault config file.


3. Generate locker private-key and public-key

        openssl genrsa -out locker-private-key.pem 2048

        openssl rsa -in locker-private-key.pem -pubout -out locker-public-key.pem


4. Generate tenant private key and public key

        openssl genrsa -out tenant-private-key.pem 2048

        openssl rsa -in tenant-private-key.pem -pubout -out tenant-public-key.pem

5. Encrypt master-key using AWS KMS

        aws kms encrypt --key-id <aws-key-id>  --plaintext $(echo -n <master-key> | base64) # paste master-key manually here

6. Encrypt the locker private-key using AWS-KMS

        aws kms encrypt --key-id <aws-key-id>  --plaintext fileb://locker-private-key.pem


7. Encrypt the tenant public-key using AWS-KMS

        aws kms encrypt --key-id <aws-key-id>  --plaintext fileb://tenant-public-key.pem


8. Encrypt the Database password using AWS-KMS

        aws kms encrypt --key-id <aws-key-id>  --plaintext fileb://tenant-public-key.pem

9. Cofigure the template available in config/. Update the encryted database password, locker_private_key, master_key and tenent_secrets public_key

10. Configure hyperswitch 

        [locker]
        host = "http://${hyperswitch-vault-private-domain}:8181"           # Locker host
        mock_locker = false                                                    # Emulate a locker locally using Postgres
        locker_enabled = true                                                 # Boolean to enable or disable saving cards in locker    

        [jwekey]
        vault_encryption_key = """"""       # Locker Public Key
        vault_private_key = """"""          # Locker Tenant Private Key

11. Database migration: export env variable DATABASE_URL=postgres://${DATABASE_USER}:${DATABASE_PASSWORD}@${DATABASE_HOST}/${DATABASE_NAME}. Run docker compose up for running migration.

12. Build the image

        docker build -t locker:latest .

13. Run the docker

        docker run --env-file .env -v ./config/production.toml:/local/config/development.toml -p 8181:8181   locker:latest
        