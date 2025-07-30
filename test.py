# /// script
# dependencies = ["requests"]
# ///

import requests

url = "https://file-share.iamaunknownpeople.workers.dev"
# url = "http://100.96.0.7:8787"

content = "test"

response = requests.post(f"{url}/upload/test_instance", data=content.encode())
print(response.status_code, response.text)

data = requests.get(f"{url}/download/test_instance:::{response.text}")
print(data.status_code, data.text)
