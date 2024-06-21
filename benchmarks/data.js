window.BENCHMARK_DATA = {
  "lastUpdate": 1718977663710,
  "repoUrl": "https://github.com/grief8/asterinas",
  "entries": {
    "sysbench-thread Benchmark": [
      {
        "commit": {
          "author": {
            "name": "Fabing Li",
            "username": "grief8",
            "email": "lifabing.lfb@antgroup.com"
          },
          "committer": {
            "name": "Fabing Li",
            "username": "grief8",
            "email": "lifabing.lfb@antgroup.com"
          },
          "id": "c8175b75a3a7a7fb688d672bc1f3318dd53276df",
          "message": "Add benchmark CI for sysbench and getpid",
          "timestamp": "2024-06-21T13:39:44Z",
          "url": "https://github.com/grief8/asterinas/commit/c8175b75a3a7a7fb688d672bc1f3318dd53276df"
        },
        "date": 1718977656987,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Linux Threads Average Latency",
            "value": "16.33",
            "unit": "ms",
            "extra": "linux_avg"
          },
          {
            "name": "Asterinas Threads Average Latency",
            "value": "501.69",
            "unit": "ms",
            "extra": "aster_avg"
          }
        ]
      }
    ]
  }
}