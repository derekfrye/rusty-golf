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
    }
});