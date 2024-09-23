function toggleRound(round) {
  const cells = document.querySelectorAll(
    `.cells.hideable[data-round="${round}"]`,
  );

  // const header = document.querySelector(`.toggle[data-round="${round}"]`);
  const smallText = document.querySelector(`.kindatiny[data-round="${round}"]`);

  const firstHeader = document.querySelectorAll(
    `.topheader.shrinkable[data-round="${round}"]`,
  );
  const secondHeaders = document.querySelectorAll(
    `.sortable.hideable[data-round="${round}"]`,
  );

  let isCollapsed = false;

  cells.forEach((cell) => {
    if (cell.style.display === "none") {
      cell.style.display = "";
    } else {
      cell.style.display = "none";
      isCollapsed = true;
    }
  });

  firstHeader.forEach((headerCell) => {
    if (isCollapsed) {
      headerCell.setAttribute("colspan", "1");
    } else {
      headerCell.setAttribute("colspan", "3");
    }
  });

  secondHeaders.forEach((headerCell) => {
    if (headerCell.style.display === "none") {
      headerCell.style.display = "";
    } else {
      headerCell.style.display = "none";
    }
  });

  // Update toggle button text
  if (smallText.textContent === "tap to expand") {
    smallText.textContent = "tap to shrink";
  } else {
    smallText.textContent = "tap to expand";
  }
}

document.addEventListener("DOMContentLoaded", function () {
  window.sortTable = function (tableId, n) {
    const table = document.getElementById(tableId);
    if (!table) {
      console.error("Table not found!");
      return;
    }
    let rows,
      switching,
      i,
      x,
      y,
      shouldSwitch,
      dir,
      switchcount = 0;
    switching = true;
    dir =
      table.rows[1].cells[n - 1].getAttribute("data-sort-dir") === "asc"
        ? "desc"
        : "asc";

    const headers = table.querySelectorAll("th.sortable");
    headers.forEach((header) => {
      header.classList.remove("sort-asc", "sort-desc");
      header.removeAttribute("data-sort-dir");
    });

    table.rows[1].cells[n - 1].setAttribute("data-sort-dir", dir);
    table.rows[1].cells[n - 1].classList.add(`sort-${dir}`);

    while (switching) {
      switching = false;
      rows = table.rows;
      for (i = 2; i < rows.length - 1; i++) {
        shouldSwitch = false;
        x = rows[i].getElementsByTagName("TD")[n + 1];
        y = rows[i + 1].getElementsByTagName("TD")[n + 1];

        let xValue = x.innerHTML.trim();
        let yValue = y.innerHTML.trim();

        if (dir == "asc") {
          if (isDate(xValue) && isDate(yValue)) {
            if (parseDate(xValue) > parseDate(yValue)) {
              shouldSwitch = true;
              break;
            }
          } else if (isNumeric(xValue) && isNumeric(yValue)) {
            if (parseFloat(xValue) > parseFloat(yValue)) {
              shouldSwitch = true;
              break;
            }
          } else if (!isDate(xValue) && !isDate(yValue)) {
            if (xValue.toLowerCase() > yValue.toLowerCase()) {
              shouldSwitch = true;
              break;
            }
          }
        } else if (dir == "desc") {
          if (isDate(xValue) && isDate(yValue)) {
            if (parseDate(xValue) < parseDate(yValue)) {
              shouldSwitch = true;
              break;
            }
          } else if (isNumeric(xValue) && isNumeric(yValue)) {
            if (parseFloat(xValue) < parseFloat(yValue)) {
              shouldSwitch = true;
              break;
            }
          } else if (!isDate(xValue) && !isDate(yValue)) {
            if (xValue.toLowerCase() < yValue.toLowerCase()) {
              shouldSwitch = true;
              break;
            }
          }
        }
      }
      if (shouldSwitch) {
        rows[i].parentNode.insertBefore(rows[i + 1], rows[i]);
        switching = true;
        switchcount++;
      } else {
        if (switchcount == 0 && dir == "asc") {
          dir = "desc";
          switching = true;
        }
      }
    }
  };

  function isNumeric(n) {
    return !isNaN(parseFloat(n)) && isFinite(n);
  }

  function isDate(dateStr) {
    const parts = dateStr.match(/(\d+)\/(\d+) (\d+):(\d+)(am|pm)/i);
    if (!parts) return false;
    return !isNaN(parseDate(dateStr));
  }

  function parseDate(dateStr) {
    const parts = dateStr.match(/(\d+)\/(\d+) (\d+):(\d+)(am|pm)/i);
    if (!parts) return new Date("Invalid Date");

    let month = parseInt(parts[1], 10) - 1;
    let day = parseInt(parts[2], 10);
    let hours = parseInt(parts[3], 10);
    const minutes = parseInt(parts[4], 10);
    const period = parts[5].toLowerCase();

    if (period === "pm" && hours < 12) hours += 12;
    if (period === "am" && hours === 12) hours = 0;

    const date = new Date();
    date.setMonth(month);
    date.setDate(day);
    date.setHours(hours);
    date.setMinutes(minutes);
    date.setSeconds(0);
    date.setMilliseconds(0);

    return date;
  }

  function initializeSorting() {
    const tables = document.querySelectorAll("table[id^='scores-table']");
    tables.forEach((table) => {
      const headers = table.querySelectorAll("th.sortable");
      headers.forEach((header, index) => {
        header.addEventListener("click", function () {
          sortTable(table.id, index);
        });
      });
    });
  }

  initializeSorting();
});
