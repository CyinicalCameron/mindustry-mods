# Scripts

This is a mixed module/crate linking python to rust to python, with the help of [pyo3](https://github.com/PyO3/pyo3) and [maturin](https://github.com/PyO3/maturin).

To build the project simply execute the following in the current directory:

```bash
maturin build
```

To install the most recent wheel built you can do something like:

```bash
pip install $(ls -Art ../target/wheels/scripts-*.whl | tail -n 1) --upgrade
```

The environmental variable `GITHUB_TOKEN` should be set to a valid Github API token when executing the main script. For example:

```bash
export GITHUB_TOKEN="<api key>"
```