# Rusty golf

Rusty golf is web app that displays a single-page scoreboard of golf scores for your family golf tournament. 

## Getting started

## First setup
```shell
git clone https://github.com/derekfrye/rusty-golf.git
cd rusty-golf/examples
podman build --tag djf/rusty-golf -f Dockerfile .
tr -dc '[:print:]' < /dev/urandom | head -c 10 > db_password.txt
podman volume create rusty_golf_pg_data
podman network create --driver bridge --subnet=10.8.0.0/16 --ipv6 --subnet fd00:ace1::/48 rusty-golf
podman-compose up -d
```

## Debugging
If you create a `.env` file in the root of this project with these parameters, it'll work with debugging. The first four are strings, and port should be a number.
```text
DB_USER=
DB_PASSWORD=
DB_HOST=
DB_NAME=
DB_PORT=
```