# htmx use

One of the nice (but somewhat complex) features we're using is [HX-Trigger](https://htmx.org/headers/hx-trigger/). Let's explain how we use that so I don't forget later.

## index

The index page uses `htmx` to load the scores.
1. When a user loads the index page, `params.js` examines the query parameters.
2. If both `yr` and `event` are in query params, then we use `htmx` to swap `innerHTML` in `#scores` with the results of a `GET` request to the `scores?yr=<param>event=<param>` page.