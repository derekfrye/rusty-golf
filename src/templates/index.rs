use maud::{html, Markup};

pub fn render_index_template() -> Markup {
    html! {
        (maud::DOCTYPE)
        head{
            meta charset="UTF-8";
            meta name="viewport" content="width=device-width, initial-scale=1.0";
            link rel="stylesheet" type="text/css" href="static/styles.css";
            title { "Family Golf" }
            script src="https://unpkg.com/htmx.org@1.9.12" {}
            script src="static/tablesort.js" {}
            script src="static/params.js" {}
        }
        body {
            h1 { "Scoreboard" }
            div id="scores" {
                img alt="Result loading..." class="htmx-indicator" width="150" src="https://htmx.org//img/bars.svg" {}
            }
        }

    }
}
