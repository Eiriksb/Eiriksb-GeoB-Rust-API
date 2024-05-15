# GeoB-Rust-API

A simple Rust API that lets you easily find the boundaries you need, whether it's for a country, state, or city. This API uses data downloaded from [https://www.geoboundaries.org](https://www.geoboundaries.org) and provides a straightforward way to access the various shapes available there.

## Running Locally

To run this API, you'll need to have Rust installed. Here's how to [Install Rust](https://www.rust-lang.org/tools/install).

Once Rust is installed, navigate to the root directory of the project and run the following commands:

```bash
cargo build
cargo run
```

This will build and start the API on your local machine (localhost).

## Running with Docker

To run the API using Docker, first build the Docker image:

```bash
docker build -t geob-api .
```

Then, run the Docker container:

```bash
docker run --name your-container-name -p 8081:8081 geob-api
```

Make sure to replace `your-container-name` with a suitable name for your container.

## Running on the Cloud

This API should run natively on cloud platforms like Fly.io or Heroku. Simply connect your project to your preferred provider and follow their setup instructions.

## First Startup

On the initial run, the API won't have the necessary data to return any boundaries. You'll need to visit `localhost-link/update-data` (replace `localhost-link` with your actual localhost URL) to update or install the data. This process will fetch the latest release from [https://www.geoboundaries.org/globalDownloads.html](https://www.geoboundaries.org/globalDownloads.html), extract all locations into separate GeoJSON files, and install them. This may take some time, depending on your internet speed and processing power. Once the installation is complete, you'll be ready to use the API to retrieve the latest geoboundaries.

## Usage

The main endpoint you'll be using is:

```
/geojson?iso3=NOR&query=Molde
```

In this example, the API will return the boundaries for Molde, Norway. The `iso3` parameter can sometimes be the full country name (e.g., Australia, Norway) but **does not** work with countries that have multiple names, such as the United States or the United Kingdom.

The `query` parameter can be the name of a state/region or a city. The API will automatically determine what you're looking for by comparing the query to all ADM1 (administrative level 1, usually states/provinces) and ADM2 (administrative level 2, usually counties/districts) locations for the specified country.

For example, to get the boundary for the Agder state in Norway, you would use:

```
/geojson?iso3=NOR&query=Agder
```
