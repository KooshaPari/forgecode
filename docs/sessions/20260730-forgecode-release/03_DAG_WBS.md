# Release WBS / DAG

```text
[R] inspect branch and dirty work
         |
         +--> [B] cargo check --------+
         +--> [T] migration test ------+--> [S] local smoke
         +--> [P] release build -------+       |
         +--> [I] isolated install ----+       +--> [E] sponsor eval build
         +--> [Q] perf scorecard ------+
                                           |
                                           v
                                  [G] remote PR + CI + main
```

Completed locally: R/B/T/P/I/Q/S. Remaining release gate: G (PR conflict/review/required checks and merge to fork `main`, then remote artifact/release verification).
