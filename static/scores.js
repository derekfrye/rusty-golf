// document.addEventListener('DOMContentLoaded', () => {
//     const buttons = document.querySelectorAll('.player-button');
//     const charts = document.querySelectorAll('.chart');

//     buttons.forEach(button => {
//         button.addEventListener('click', () => {
//             const selectedPlayer = button.getAttribute('data-player');

//             // Update button styles
//             buttons.forEach(btn => btn.classList.remove('selected'));
//             button.classList.add('selected');

//             // Show/hide corresponding chart
//             charts.forEach(chart => {
//                 if (chart.getAttribute('data-player') === selectedPlayer) {
//                     chart.classList.add('visible');
//                 } else {
//                     chart.classList.remove('visible');
//                 }
//             });
//         });
//     });
// });

function toggleAllPlayersDetailDiv() {
    const playerDetailsDiv = document.querySelector('.playerdetailsdiv');
    if (playerDetailsDiv) {
      if (playerDetailsDiv.style.display === 'none') {
        playerDetailsDiv.style.display = 'block';
      } else {
        playerDetailsDiv.style.display = 'none';
      }
    }
  }


document.addEventListener('click', (event) => {
    if (event.target.classList.contains('player-button')) {
        const button = event.target;
        const selectedPlayer = button.getAttribute('data-player');

        // Update button styles
        document.querySelectorAll('.player-button').forEach(btn => btn.classList.remove('selected'));
        button.classList.add('selected');

        // Show/hide corresponding chart
        document.querySelectorAll('.chart').forEach(chart => {
            if (chart.getAttribute('data-player') === selectedPlayer) {
                chart.classList.add('visible');
                chart.classList.remove('hidden');
            } else {
                chart.classList.remove('visible');
                chart.classList.add('hidden');
            }
        });


        document.querySelectorAll('.linescore-container').forEach(lsc => {
            if (lsc.getAttribute('data-player') === selectedPlayer) {
              lsc.classList.add('visible');
              lsc.classList.remove('hidden');
            } else {
              lsc.classList.remove('visible');
              lsc.classList.add('hidden');
            }
          });

        // Show/hide corresponding player row details
        document.querySelectorAll('.playerrow').forEach(chart => {
            if (chart.getAttribute('data-player') === selectedPlayer) {
                chart.classList.add('visible');
                chart.classList.remove('hidden');
            } else {
                chart.classList.remove('visible');
                chart.classList.add('hidden');
            }
        });

        // Show/hide corresponding player row details
        const elements = document.querySelectorAll('p.playerdetailsmsg');
        elements.forEach(element => {
            // Update the text content of the <p> element
            element.textContent = `Showing details for ${selectedPlayer}.`;
    
            // Create the link element
            const link = document.createElement('a');
            link.href = '#'; // Use '#' to prevent navigation
            link.textContent = ' Click here to reset filter.';
            link.style.cursor = 'pointer'; // Makes it look like a clickable link
    
            // Add a click event listener to the link
            link.addEventListener('click', (event) => {
                event.preventDefault(); // Prevent default link behavior
                
                // Change the original text back
                // keep in sync with view/score.rs
                element.textContent = 'Showing details for all players. You can further filter by clicking links above.';
    
                // Find all elements with class 'playerrow' and update their visibility
                const rows = document.querySelectorAll('.playerrow');
                rows.forEach(row => {
                    row.classList.add('visible');
                    row.classList.remove('hidden');
                });
            });
    
            // Append the link to the <p> element
            element.appendChild(link);
        });
    }

    if (event.target.classList.contains('linescore-round-button')) {
        const button = event.target;
        const selectedRound = button.getAttribute('data-round');

        // Update round button styles
        document.querySelectorAll('.linescore-round-button').forEach(btn => btn.classList.remove('selected'));
        button.classList.add('selected');

        document.querySelectorAll(`.linescore-round-button[data-round="${selectedRound}"]`).forEach(btn => {
            btn.classList.add('selected');
        });

        // Show/hide corresponding table rows
        filterRound(selectedRound);
    }
});

function filterRound(round) {
    // Iterate over all table rows
    document.querySelectorAll('.linescore-table tbody tr, .linescore-table thead tr th div').forEach(tr => {
        if (tr.getAttribute('data-round') === String(round)) {
            tr.classList.remove('hidden');
        } else {
            tr.classList.add('hidden');
        }
    });
}