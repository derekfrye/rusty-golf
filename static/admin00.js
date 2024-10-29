document.addEventListener('DOMContentLoaded', function () {
    document.getElementById('create-missing-tables').addEventListener('click', function () {
        var button = this;
        // Disable the button to prevent multiple clicks
        button.disabled = true;
        // Get the data from the script tag
        var scriptTag = document.getElementById('admin00_missing_tables').textContent;
        const unescapedData = scriptTag.replace(/&quot;/g, '"');
        var admin_missing_table_json = JSON.parse(unescapedData);

        var timesRunContent = document.getElementById('times_run').textContent;
        const unescapedData1 = timesRunContent.replace(/&quot;/g, '"');
        var times_run_as_json = JSON.parse(unescapedData1);

        // Get the token from the URL
        var urlParams = new URLSearchParams(window.location.search);
        var token = urlParams.get('token');
        var page = urlParams.get('p');

        // Prepare the params for the AJAX call, including the token
        var params = {
            // name your params how they'll appear in your router
            admin00_missing_tables: JSON.stringify(admin_missing_table_json),
            times_run: JSON.stringify(times_run_as_json),
            token: token, // Add the token to the params
            p: page
        };

        var queryString = new URLSearchParams(params).toString();

        htmx.ajax('GET', 'admin?' + queryString, {
            target: '#create-table-results',
            swap: 'innerHTML',
        });
    });

    document.addEventListener("reenablebutton", function (evt) {
        // fired by HX-Trigger header
        var button = document.getElementById('create-missing-tables');
        button.disabled = false;
        // alert("myEvent was triggered!");
    });

    document.addEventListener("times_run", function (evt) {
        // evt.detail will contain the payload passed in the HX-Trigger header
        const timesRun = evt.detail.value;

        // Find the element by its id
        const timesRunElement = document.getElementById("times_run");

        // construct a json array now
        var data = { "times_run": timesRun };

        // Update the inner HTML of the element with the value from the event
        if (timesRunElement) {
            timesRunElement.innerHTML = JSON.stringify(data);
        }
    });

});