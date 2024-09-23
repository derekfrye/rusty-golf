document.addEventListener("DOMContentLoaded", function () {
    function getQueryParams() {
        const params = new URLSearchParams(window.location.search);
        const eventId = params.get('event');
        const yr = params.get('yr');
        //   console.log("URL Parameters:", Array.from(params.entries())); // Log all URL parameters
        return { eventId, yr };
    }

    const { eventId, yr } = getQueryParams();
    // console.log("Extracted eventId:", eventId, "Extracted yr:", yr); // Log extracted values

    if (eventId && yr) {
        const scoresUrl = `scores?event=${eventId}&yr=${yr}`;
        htmx.ajax('GET', scoresUrl, { target: '#scores', swap: 'innerHTML' });

    } else {
        console.error("event or yr parameters are missing in the URL.");
    }
});