CREATE TRIGGER eup_statistic_before_update
BEFORE UPDATE ON eup_statistic
FOR EACH ROW
BEGIN
    INSERT INTO eup_statistic_hx (
        event_espn_id,
        golfer_espn_id,
        eup_id,
        grp,
        rounds,
        round_scores,
        tee_times,
        holes_completed_by_round,
        line_scores,
        total_score,
        ins_ts
    )
    VALUES (
        OLD.event_espn_id,
        OLD.golfer_espn_id,
        OLD.eup_id,
        OLD.grp,
        OLD.rounds,
        OLD.round_scores,
        OLD.tee_times,
        OLD.holes_completed_by_round,
        OLD.line_scores,
        OLD.total_score,
        OLD.ins_ts
    );
END;