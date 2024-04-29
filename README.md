# GeoB-Rust-API
A Simple Rust API that lets you easily find the bounderies you want Country State or City. This uses data downloaded from https://www.geoboundaries.org. This API is just a simple way of returning one of the many shapes that are provided by geobounderies.org.

## Running locally

To run this API you will need to have Rust installed her is how to [Install Rust](https://www.rust-lang.org/tools/install) 

After this is done you can inside the root directory run
```sh
cargo build
```
then
```sh
cargo run
```

to run it! This will start the API on localhost.

## Running with docker

To run the API using docker you will first have to build it using: 
```sh
docker build -t geob-api .
```

and then 
```sh
docker run --name your-container-name -p 8081:8081 geob-api
```

## Running on the Cloud
Currently this should just run using either fly.io or heroku pretty much nativley. So it would just need to be connected to one of these services and setup as the provider want you too.

# First Startup
During Inital run the API wont have the data it needs to return any bounderies. You will have to go to the localhost-link/update-data to update OR install the data. This will fetch the latest release from https://www.geoboundaries.org/globalDownloads.html, Extract all locations to seperate geoJson files and after this install (Which might take some time depending on internet speed and processing speed) You will be ready to run it and retunr the lates geo bonderies!
