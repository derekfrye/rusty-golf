// Handle adding new rows with a smooth animation
document.getElementById('add-row').addEventListener('click', function () {
    var tbody = document.getElementById('table-body');
    var newRow = tbody.rows[0].cloneNode(true);

    // Add delete button to the new row
    var deleteCell = document.createElement('td');
    var deleteButton = document.createElement('button');
    deleteButton.innerHTML = 'Delete';
    deleteButton.classList.add('delete-row');
    deleteCell.appendChild(deleteButton);
    newRow.appendChild(deleteCell);

    tbody.appendChild(newRow);
    newRow.classList.add('animate');
});

// Handle row deletion with a smooth animation
document.getElementById('table-body').addEventListener('click', function (event) {
    if (event.target && event.target.classList.contains('delete-row')) {
        var row = event.target.closest('tr');
        row.classList.add('animate-out'); // Add the class for fade-out animation
        setTimeout(function () {
            row.remove(); // Remove the row after the animation ends
        }, 300); // Adjust the time to match your CSS animation duration
    }
});

// Handle form submission using htmx.ajax
document.getElementById('submit').addEventListener('click', function () {
    var rows = document.querySelectorAll('#table-body tr');
    var data = [];
    rows.forEach(function (row, index) {
        var playerSelect = row.querySelector('.player-select');
        var bettorSelect = row.querySelector('.bettor-select');
        var roundSelect = row.querySelector('.round-select');
        var rowData = {
            row_entry: index,
            'player.id': playerSelect.value,
            'bettor.id': bettorSelect.value,
            round: roundSelect.value
        };
        data.push(rowData);
    });

    // Get the token from the URL
    var urlParams = new URLSearchParams(window.location.search);
    var token = urlParams.get('token');

    // Prepare the params for the ajax call, including the token
    var params = {
        data: JSON.stringify(data),
        token: token // Add the token to the params
    };

    var queryString = new URLSearchParams(params).toString();

    htmx.ajax('GET', 'admin?' + queryString, {
        target: '#results',
        swap: 'innerHTML'
    });
});

// Handle admin00 table creation
document.getElementById('create-missing-tables').addEventListener('click', function () {
    var button = this;
    // Disable the button to prevent multiple clicks
    button.disabled = true;
    // Get the data from the script tag
    var scriptTag = document.getElementById('admin00_missing_tables').textContent;
    const unescapedData = scriptTag.replace(/&quot;/g, '"');
    var data = JSON.parse(unescapedData);


    var timesRunContent = document.getElementById('times_run').textContent;
    const unescapedData1 = timesRunContent.replace(/&quot;/g, '"');
    var data1 = JSON.parse(unescapedData1);

    // Get the token from the URL
    var urlParams = new URLSearchParams(window.location.search);
    var token = urlParams.get('token');

    // Prepare the params for the AJAX call, including the token
    var params = {
        // name your params how they'll appear in your router
        admin00_missing_tables: JSON.stringify(data),
        times_run: JSON.stringify(data1),
        token: token // Add the token to the params
    };

    var queryString = new URLSearchParams(params).toString();

    htmx.ajax('GET', 'admin?' + queryString, {
        target: '#create-table-results',
        swap: 'innerHTML',
    });
});

document.body.addEventListener("reenablebutton", function (evt) {
    var button = document.getElementById('create-missing-tables');
    button.disabled = false;
    // alert("myEvent was triggered!");
});

document.body.addEventListener("times_run", function (evt) {
    // evt.detail will contain the payload passed in the HX-Trigger header
    const timesRun = evt.detail.value;
    //  alert("timesRun: " + timesRun);
    // const unescapedData = timesRun.replace(/&quot;/g, '"');
    // var data = JSON.parse(unescapedData);
    // var timesRunx = data.times_run;
    
    // Find the element by its id
    const timesRunElement = document.getElementById("times_run");

    // construct a json array now
    var data = { "times_run": timesRun };

    // Update the inner HTML of the element with the value from the event
    if (timesRunElement) {
        // timesRunElement.innerHTML = `You've run this ${timesRun} times`;
        // console.log(data);
        // let's stringify the data
        timesRunElement.innerHTML = JSON.stringify(data);
        // timesRunElement.innerHTML = data.toString();
    }
});