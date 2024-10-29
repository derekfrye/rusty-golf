# Rusty golf

Rusty golf is web app that displays a single-page scoreboard of golf scores for your family golf tournament. 

## Getting started

## First setup
```shell
git clone https://github.com/derekfrye/rusty-golf.git
cd rusty-golf/examples
podman build --tag djf/rusty-golf -f Dockerfile .
(tr -dc '[:print:]' < /dev/urandom | head -c 14; echo) > db_password.txt
chmod 600 db_password.txt
podman volume create rusty_golf_pg_data
podman network create --driver bridge --subnet=10.8.0.0/16 --ipv6 --subnet fd00:ace1::/48 rusty-golf
podman-compose up -d
python -m webbrowser http://localhost:9000/admin
```

## Debugging
If you create a `.env` file in the *root* of this project[^1], it's a great way to do debugging (vscode debugging works great). Note, there's a `.dockerignore` which will exclude it from container build. Specify the `<values>` below in the `.env` based on your needs.
```text
DB_USER=<string>
DB_PASSWORD=<string>
DB_HOST=<ip4 is tested, but ipv6 might work, idk>
DB_NAME=<string>
DB_PORT=<integers>
TOKEN=<14-character, composed of ascii alphabet characters and/or numbers>
```

[^1]: Where's the *root* of the project? Right after you clone from github, create a `.env` file alongside the `LICENSE` file like so:
    <pre>
    .
    ├── Cargo.toml
    ├── Dockerfile
    ├── docs
    │   └── README.md
    <b>├── .env</b>
    ├── examples
    │   ├── docker-compose.yml
    │   ├── Dockerfile -> ../Dockerfile
    │   └── init_db.sql
    ├── LICENSE
    ├── src
    │   ├── controller
    ...
    </pre>