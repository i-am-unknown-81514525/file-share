# /// script
# dependencies = ["requests"]
# ///

import requests
import time
import math

def fmt_time(seconds):
    if seconds < 1:
        return f"{seconds * 1000:.2f}ms"
    else:
        return f"{seconds:.4f}s"

url = "https://file-share.iamaunknownpeople.workers.dev"
# url = "http://100.96.0.7:8787"

content = "test"*100000

l1 = []
l2 = []
run_count = 0

try:
    while True:
        run_count += 1
        t1 = time.perf_counter()
        response = requests.post(f"{url}/upload/test_instance", data=content.encode())
        t2 = time.perf_counter()
        print(response.status_code, response.text, t2-t1)
        l1.append(t2-t1)

        t3 = time.perf_counter()
        data = requests.get(f"{url}/download/test_instance:::{response.text}")
        t4 = time.perf_counter()
        print(data.status_code, data.text[:10], t4-t3)
        l2.append(t4-t3)
        time.sleep(1)

except KeyboardInterrupt:
    if run_count > 0:
        print(f"Total runs: {run_count}")
        print(f"Average Upload Time: {fmt_time(sum(l1)/len(l1))}, stddev: {fmt_time(math.sqrt(sum([(x - sum(l1)/len(l1))**2 for x in l1])/len(l1)))}")
        print(f"Average Download Time: {fmt_time(sum(l2)/len(l2))}, stddev: {fmt_time(math.sqrt(sum([(x - sum(l2)/len(l2))**2 for x in l2])/len(l2)))}")
    else:
        print("No runs were completed.")
