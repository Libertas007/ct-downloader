# ČT Downloader

This app enables you to download a film or a TV series from [iVysílání](https://www.ceskatelevize.cz/ivysilani/).

> Built for educational purposes only.

## Prerequisities

You shall have `ffmpeg` installed on your computer.

## Building

Just run `cargo build`.

## Usage

1. On iVysílání website, copy the URL to the film or TV series.
1. Run `ct-downloader <URL>`
1. The downloading will start automatically; if a TV series is selected, you'll be prompted with a selection of episodes to download.

> Select concurrent episode download count wisely, the application downloads **ALL** the episodes at once, thus reducing overall download speed and increasing hardware usage.
