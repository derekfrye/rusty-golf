document.addEventListener("DOMContentLoaded", function() {
    // Get the token and page parameters from the URL
    var urlParams = new URLSearchParams(window.location.search);
    var token = urlParams.get('token');
    var page = "01";
    var x = "x";

    // Prepare the params for the AJAX call, including the token
    var params = {
        token: token,
        p: page,
        from_landing_page: x,
    };

    var queryString = new URLSearchParams(params).toString();

    // Use htmx to make the GET request and swap the content into landing_grid_1
    htmx.ajax('GET', 'admin?' + queryString, {
        target: '#landing_grid_1',
        swap: 'innerHTML',
    });
});
