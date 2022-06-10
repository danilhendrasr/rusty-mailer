-- Add migration script here

INSERT INTO
    users(id, username, password_hash)
VALUES
    (
        '84aeda43-6d16-4827-bed5-381ec86a11e7',
        'admin',
        '$argon2id$v=19$m=15000,t=2,p=1$U95fTxnptQjE9KTrvbdrhA$QbkqdUvo1Zp1aLPgvwVQrZHio72WRoiXsnhVAEZRDKU'
    );