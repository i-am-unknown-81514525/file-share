### FileShare

----
A rust written Cloudflare Worker which allow file sharing with a code, and a frontend made with python marimo/html.

I implement this for mainly game to allow save sharing between users (hence the namespace to prevent people upload a save for a game and download a different one by Instance ID)

It don't store the filename of the files and the application using it **should** know the filename of it (although not implement in the UI as it wouldn't know)


### API Endpoint: