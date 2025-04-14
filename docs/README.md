# Rusty golf

Rusty golf is web app that displays a single-page scoreboard for your family golf tournaments. It stores configuration in a postgres or sqlite database (the examples here are written assuming you're using sqlite). There's an admin panel to monitor the functionality of the database for this app.

## Getting started

## First setup
```shell
git clone https://github.com/derekfrye/rusty-golf.git
cd rusty-golf
podman build --tag djf/rusty-golf -f Dockerfile .

# (Optional) you'll need these if using the example docker-compose.yml
podman volume create rusty_golf_data
podman network create --driver bridge --subnet=10.8.0.0/16 --ipv6 --subnet fd00:ace1::/48 rusty-golf
cd examples
podman-compose up -d
python -m webbrowser http://localhost:9000/?event=401703504&yr=2025
```

## Adding a tournament

Wouldn't it be nice of the admin panel automated this?[^1] 😆

1. Go [here](https://site.web.api.espn.com/apis/v2/scoreboard/header?sport=golf&league=pga&region=us&lang=en&contentorigin=espn), get the event ID.
2. Go here https://site.web.api.espn.com/apis/site/v2/sports/golf/pga/leaderboard/players?region=us&lang=en&event=&lt;eventId&gt;, find all the golfers you want to include.
3. Populate [db_prefill.json](tests/test5_dbprefill.json) with the data you need for your tournament.
4. Restart with podman-compose; if using the [example docker-compose.yml](examples/docker-compose.yml), it'll read the db_prefill.json and load the data into the sqlite database.

## Postgresql Debugging
If you want to use postgresql, and you want to debug the project, create a `.env` file in the *root* of this project[^2]. Vscode debugging has been tested. Your `.env` will be excluded by `.dockerignore`. Specify your postgres `<values>` below based on your needs. If you're using the example podman compose file, make sure your DB_PORT and DB_HOST match entries in `examples/docker-compose.yml`.
```text
DB_USER=<string>
DB_PASSWORD=<string>
DB_HOST=<ip4 is tested, but ipv6 might work, idk>
DB_NAME=<string>
DB_PORT=<integers>
TOKEN=<14-character, composed of ascii alphabet characters and/or numbers>
```

## How we use `htmx`

See [htmx documentation](htmx.md) for how we use `htmx`.

[^1]: Well.. *eventually* we'll use the admin interface for configuration. Right now that's just an aspiration.

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