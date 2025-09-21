duplicacy-du
============

Creates a [Ncdu JSON Export](https://dev.yorhel.nl/ncdu/jsonfmt) file with what is backed up by [Duplicacy](https://github.com/gilbertchen/duplicacy).

## Usage

Install:

```
cargo install --git https://github.com/p2004a/duplicacy-du
```

Run:

```
duplicacy -debug -no-script -log backup -enum-only | duplicacy-du | ncdu -f -
```

The above uses [`ncdu`](https://dev.yorhel.nl/ncdu) but other similar programs like [`gdu`](https://github.com/dundee/gdu) also support this export format.
