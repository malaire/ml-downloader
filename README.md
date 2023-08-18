## ml-downloader

Simple blocking downloader, featuring:

- retries with custom delays
- custom interval between successful downloads for rate limiting
- hash check (optional)
- based on [reqwest](https://crates.io/crates/reqwest)

## Examples

### Simple usage

Create [`Downloader`] with default configuration and then download one file.

```no_run
use ml_downloader::Downloader;

let mut downloader = Downloader::new()?;
let bytes = downloader.get("https://example.com/").send()?;
# Ok::<(), ml_downloader::Error>(())
```

### Custom configuration

Create [`Downloader`] with
- `"foobar/1.0"` as `USER_AGENT`
- `1.0 - 1.1` seconds interval between successful downloads
- two retries after failed download
    - `2.0 - 2.2` seconds delay after initial failure
    - `5.0 - 5.5` seconds delay after 2nd failure

```rust
use ml_downloader::Downloader;

let mut downloader = Downloader::builder()
    .reqwest(|cb| cb.user_agent("foobar/1.0"))
    .interval(1.0, 1.1)
    .retry_delays(&[(2.0, 2.2), (5.0, 5.5)])
    .build()?;

# Ok::<(), ml_downloader::Error>(())
```

[`Downloader`]: https://docs.rs/ml-downloader/0.1.1/ml_downloader/struct.Downloader.html
