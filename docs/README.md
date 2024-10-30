# Rusty golf

Rusty golf is web app that displays a single-page scoreboard for your family golf tournaments. It stores configuration in a postgres database. There's an admin panel, so there's no need to raw-dog the configuration SQL[^1].

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
If you create a `.env` file in the *root* of this project[^2], it's a great way to do debugging (vscode debugging works great). Note, there's a `.dockerignore` which will exclude it from container build. Specify the `<values>` below in the `.env` based on your needs. If you're using the setup steps above, make sure your DB_PORT and DB_HOST match the entries in `examples/docker-compose.yml`.
```text
DB_USER=<string>
DB_PASSWORD=<string>
DB_HOST=<ip4 is tested, but ipv6 might work, idk>
DB_NAME=<string>
DB_PORT=<integers>
TOKEN=<14-character, composed of ascii alphabet characters and/or numbers>
```

[^1]: Well.. *eventually* you'll exclusively use the admin interface for configuration. Right now that's just an aspiration.

[^2]: Where's the *root* of the project? The root of the project is alongside the `LICENSE` file. Create a `.env` file there for debugging with VScode.
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