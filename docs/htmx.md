# htmx use

This helps document the projects use of `htmx`. One of the nice (but somewhat complex) features we're using is [HX-Trigger](https://htmx.org/headers/hx-trigger/). Let's explain how we use that so I don't forget later.

## admin

### Setting up some html in an admin page

1. `admin.rs` routes on url paramater `p`. When `p` is set to `01`, we use fns in `admin01.rs`
2. A user navigates to `p=00` page. That calls `do_tables_exist()` within `admin01.rs`, which makes a "Create missing tables" button (if the postgres databse is missing tables).
3. `do_tables_exist()` also creates a `script type="application/json"` block with `id` set to `admin01_missing_tables`, and we store a json list of missing tables in that script block.

### JavaScript to handle events

1. Notice that `admin01.rs` also includes `script src="static/admin01.js"`. When the page loaded, it hooked up a listener to the `click` event of "Create missing tables" button.
2. Also notice two other JavaScriptfunctions named `reenablebutton` and `times_run`. `htmx` is going hook up an event to listen for a header with these names.

### GET request to `admin01`, and an html header to trigger events

1. When the user clicks the "Create missing tables" button, `admin01.js` submits a GET request back to `admin.rs`. Part of that GET request are the contents of `admin01_missing_tables`.
2. When `admin.rs` receives `p=00` along with `admin01_missing_tables`, it's going to route to `admin01.rs` fn `get_html_for_create_tables()`.
3. fn `get_html_for_create_tables()` calls some other functions which parse the json etc., but importantly, inserts an `HX-Trigger` including multiple json entries. 
4. When the html header is received by the client, the two named JavaScriptevents will trigger, re-enabling the "Create missing tables" button, and storing the number of times the user has pressed the button in a `script type="application/json"` with id `times_run`.

## index

The index page uses `htmx` to load the scores.
1. When a user loads the index page, `params.js` examines the query parameters.
2. If both `yr` and `event` are in query params, then we use `htmx` to swap `innerHTML` in `#scores` with the results of a `GET` request to the `scores?yr=<param>event=<param>` page.