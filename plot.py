#!/usr/bin/env -S pipx run
# /// script
# requires-python = ">=3.9"
# dependencies = ["click ~= 8.0", "matplotlib ~= 3.9"]
# ///

from __future__ import annotations
from collections import defaultdict
import json
from pathlib import Path
import click
import matplotlib.pyplot as plt


@click.command()
@click.option("-v", "--view", is_flag=True)
@click.argument(
    "infile",
    type=click.Path(dir_okay=False, exists=True, readable=True, path_type=Path),
    nargs=-1,
)
def main(infile: tuple[Path, ...], view: bool) -> None:
    if view and len(infile) != 1:
        raise click.UsageError("--view can only be used with exactly one argument")
    for fn in infile:
        data = defaultdict(list)
        with fn.open() as fp:
            for trav in json.load(fp)["traversals"]:
                workers = trav["workers"]
                for duration in trav["request_times"]:
                    data[workers].append(
                        duration["secs"] + duration["nanos"] / 1_000_000_000
                    )
        fig = plt.figure(figsize=(10, 5), layout="constrained")
        ax = fig.subplots()
        ax.boxplot(
            list(data.values()),
            positions=list(data.keys()),
            vert=True,
            widths=0.5,
            showmeans=True,
            meanline=False,
            showfliers=False,
            manage_ticks=False,
            medianprops={"color": "red", "linewidth": 0.5},
            # boxprops={"facecolor": "C0", "edgecolor": "white", "linewidth": 0.5},
            whiskerprops={"color": "C0", "linewidth": 1.5},
            capprops={"color": "C0", "linewidth": 1.5},
            meanprops={"marker": "*", "markersize": 5},
        )
        ax.set_xlabel("Workers")
        ax.set_ylabel("Request Time (s)")
        if view:
            plt.show()
        else:
            fig.savefig(fn.with_suffix(".png"))


if __name__ == "__main__":
    main()
