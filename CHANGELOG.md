# 0.3.0 (2021-05-20)

- Added ``--save`` option to write output to file
- Now it fails when run as root


# 0.2.1 (2021-02-22)

- Enabled _vendored_ feature for `openssl-sys` crate to allow for musl builds


# 0.2.0 (2021-02-21)

- Added `--sort-mirrors-by` option to control how mirrors are initially sorted
  within the country, `store_asc` by default. The full list of options is:
  _score_asc, score_desc, delay_asc, delay_desc, random_

- Added `--protocol` option to control acceptable protocols `https`, `http`. Both
  both are enabled by default. Rsync not supported.


# 0.1.0 (2021-01-17)

Initial.
