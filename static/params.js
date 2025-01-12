document.addEventListener("DOMContentLoaded", function () {
    function getAllQueryParams() {
        const params = new URLSearchParams(window.location.search);
        const queryParams = {};
        for (const [key, value] of params.entries()) {
            queryParams[key] = value;
        }
        return queryParams;
    }

    const queryParams = getAllQueryParams();
    const { event, yr, expanded, json, cache } = queryParams;

    if (event && yr) {
        let scoresUrl = `scores?event=${event}&yr=${yr}`;
        if (expanded) scoresUrl += `&expanded=${expanded}`;
        if (json) scoresUrl += `&json=${json}`;
        if (cache) scoresUrl += `&cache=${cache}`;

        htmx.ajax('GET', scoresUrl, { target: '#scores', swap: 'innerHTML' });
    } else {
        console.error("event or yr parameters are missing in the URL.");
    }
});