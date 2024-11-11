document.addEventListener('DOMContentLoaded', function () {
    // Check if 'create-missing-tables' element exists and add event listener if so
    var createMissingTablesButton = document.getElementById('create-missing-tables');
    if (createMissingTablesButton) {
        createMissingTablesButton.addEventListener('click', function () {
            var button = this;
            // Disable the button to prevent multiple clicks
            button.disabled = true;

            // Get the data from the script tag
            var scriptTag = document.getElementById('admin01_missing_tables').textContent;
            const unescapedData = scriptTag.replace(/&quot;/g, '"');
            var admin_missing_table_json = JSON.parse(unescapedData);

            // Check if 'times_run' element exists before using it
            var timesRunContent = document.getElementById('times_run')?.textContent || "{}";
            const unescapedData1 = timesRunContent.replace(/&quot;/g, '"');
            var times_run_as_json = JSON.parse(unescapedData1);

            // Get the token from the URL
            var urlParams = new URLSearchParams(window.location.search);
            var token = urlParams.get('token');
            var page = urlParams.get('p');

            // Prepare the params for the AJAX call, including the token
            var params = {
                admin01_missing_tables: JSON.stringify(admin_missing_table_json),
                times_run: JSON.stringify(times_run_as_json),
                token: token,
                p: page
            };

            var queryString = new URLSearchParams(params).toString();

            htmx.ajax('GET', 'admin?' + queryString, {
                target: '#create-table-results',
                swap: 'innerHTML',
            });
        });
    }

    // Check if 'create-missing-tables' element exists and add 'reenablebutton' listener if so
    if (createMissingTablesButton) {
        document.addEventListener("reenablebutton", function (evt) {
            // fired by HX-Trigger header
            var button = document.getElementById('create-missing-tables');
            if (button) {
                button.disabled = false;
            }
        });
    }

    // Check if 'times_run' element exists and add 'times_run' listener if so
    var timesRunElement = document.getElementById("times_run");
    if (timesRunElement) {
        document.addEventListener("times_run", function (evt) {
            // evt.detail will contain the payload passed in the HX-Trigger header
            const timesRun = evt.detail.value;

            // Construct a JSON object with the times_run value
            var data = { "times_run": timesRun };

            // Update the inner HTML of the element with the value from the event
            timesRunElement.innerHTML = JSON.stringify(data);
        });
    }
});
