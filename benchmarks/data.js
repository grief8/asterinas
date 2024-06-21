window.BENCHMARK_DATA = {
  "lastUpdate": 1718979335059,
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
    ],
    "getpid Benchmark": [
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
        "date": 1718978882603,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Linux Syscall Average Latency",
            "value": "109",
            "unit": "ns",
            "extra": "linux_avg"
          },
          {
            "name": "Asterinas Syscall Average Latency",
            "value": "475",
            "unit": "ns",
            "extra": "aster_avg"
          }
        ]
      }
    ],
    "sysbench-cpu Benchmark": [
      {
        "commit": {
          "author": {
            "name": "Fabing Li",
            "username": "grief8",
            "email": "lifabing.lfb@antgroup.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "5dc579236fed42ab32c24816984e688682a0bd49",
          "message": "Update benchmarks.yml",
          "timestamp": "2024-06-21T14:09:18Z",
          "url": "https://github.com/grief8/asterinas/commit/5dc579236fed42ab32c24816984e688682a0bd49"
        },
        "date": 1718979329332,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Linux CPU Average Latency",
            "value": "3.83",
            "unit": "ms",
            "extra": "linux_avg"
          },
          {
            "name": "Asterinas CPU Average Latency",
            "value": "3.83",
            "unit": "ms",
            "extra": "aster_avg"
          }
        ]
      }
    ]
  }
}