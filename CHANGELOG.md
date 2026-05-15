# Changelog

## NEXT
- rename `Downloader::get` to `url` **breaking**
- rename `RequestBuilder::send` to `get` and change return value to `BytesResponse` **breaking**
- rename `RequestBuilder::hash` to `verify_hash` **breaking**
- add `RequestBuilder::calculate_hash`
- add `RequestBuilder::save_to_file`

## 0.2.0 - 2025-11-21
- add `DownloaderBuilder::delay`
- update dependencies

## 0.1.1 - 2023-08-18
- fix: Invalid URL causes error instead of crash
- update dependencies

## 0.1.0 - 2022-02-27
- First public version.
