#!/usr/bin/env bash

servers=("http://89.46.234.90:8000"
         "http://185.195.64.131:8000"
         "http://192.71.227.163:8000"
         "http://37.235.55.107:8000"
         "http://151.236.22.184:8000"
         "http://192.121.46.48:8000"
         "http://185.193.48.172:8000"
         "http://162.252.175.161:8000"
         "http://151.236.22.151:8000"
         "http://92.243.64.226:8000"
         "http://151.236.21.31:8000"
         "http://185.195.66.254:8000"
         "http://194.14.208.138:8000"
         "http://92.243.65.86:8000"
         "http://192.71.213.214:8000"
         "http://91.132.92.60:8000")

CUR_ITER=0
for host in "${servers[@]}"; do
    echo "Testing server: $host"
    locust -f locustfile.py --headless -u 30 -r 10 --run-time 1m --host=$host --csv reports/node${CUR_ITER}
    echo "Finished testing server: $host"
    ((CUR_ITER++))
done