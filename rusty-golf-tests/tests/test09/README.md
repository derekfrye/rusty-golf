Fixture provenance for test09_setup_oneshot

These JSON files are ESPN responses captured as golden masters for offline
tests:
- scoreboard.json from the ESPN scoreboard header endpoint.
- event_401580351.json and event_401580360.json from the ESPN leaderboard
  endpoint for those events.

To refresh:
1) Re-download the three JSON files from ESPN:
   curl -sS "https://site.web.api.espn.com/apis/v2/scoreboard/header?sport=golf&league=pga&region=us&lang=en&contentorigin=espn" -o scoreboard.json
   curl -sS "https://site.web.api.espn.com/apis/site/v2/sports/golf/pga/leaderboard/players?region=us&lang=en&event=401580351" -o event_401580351.json
   curl -sS "https://site.web.api.espn.com/apis/site/v2/sports/golf/pga/leaderboard/players?region=us&lang=en&event=401580360" -o event_401580360.json
2) Keep filenames unchanged so the fixture client can find them.
