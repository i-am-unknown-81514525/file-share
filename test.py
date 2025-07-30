# /// script
# dependencies = ["requests"]
# ///

import requests

url = "https://file-share.iamaunknownpeople.workers.dev"

content = "test"

response = requests.post(f"{url}/upload/test_instance", data=content.encode())
print(response.status_code, response.text)