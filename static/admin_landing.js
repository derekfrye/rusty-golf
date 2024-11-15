document.addEventListener("DOMContentLoaded", function () {
    // Get the token and page parameters from the URL
    var urlParams = new URLSearchParams(window.location.search);
    var token = urlParams.get('token');
    var page = "01"; // has to match the lookup value in AdminPage.parse
    var x = "x";

    // Prepare the params for the AJAX call, including the token
    var params = {
        token: token,
        p: page,
        from_landing_page_tables: x, // the value of this param doesn't matter, but its matched in router.rs
    };

    var queryString = new URLSearchParams(params).toString();

    // Use htmx to make the GET request and swap the content into landing_grid_1
    htmx.ajax('GET', 'admin?' + queryString, {
        target: '#cell_body_1',
        swap: 'innerHTML',
    });


});

document.addEventListener("DOMContentLoaded", function () {
    // Get the token and page parameters from the URL
    var urlParams = new URLSearchParams(window.location.search);
    var token = urlParams.get('token');
    var page = "01"; // has to match the lookup value in AdminPage.parse
    var x = "x";

    // Prepare the params for the AJAX call, including the token
    var params = {
        token: token,
        p: page,
        from_landing_page_constraints: x, // the value of this param doesn't matter, but its matched in router.rs
    };

    var queryString = new URLSearchParams(params).toString();

    // Use htmx to make the GET request and swap the content into landing_grid_1
    htmx.ajax('GET', 'admin?' + queryString, {
        target: '#cell_body_2',
        swap: 'innerHTML',
    });


});
