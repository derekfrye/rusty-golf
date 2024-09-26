// Handle adding new rows with a smooth animation
document.getElementById('add-row').addEventListener('click', function() {
    var tbody = document.getElementById('table-body');
    var newRow = tbody.rows[0].cloneNode(true);
    tbody.appendChild(newRow);
    newRow.classList.add('animate');
});

// Handle form submission using htmx.ajax
document.getElementById('submit').addEventListener('click', function() {
    var rows = document.querySelectorAll('#table-body tr');
    var data = [];
    rows.forEach(function(row, index) {
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

    var params = {
        data: JSON.stringify(data)
    };

    var queryString = new URLSearchParams(params).toString();

    htmx.ajax('GET', '/admin?' + queryString, {
        target: '#results',
        swap: 'innerHTML'
    });
});