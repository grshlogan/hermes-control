# Hermes Control vLLM Runtime

This directory is the software-owned vLLM runtime boundary for Hermes Control.
It may contain a Python virtual environment, caches, logs, and runtime scripts.
The default vLLM socket/temp directory stays under WSL `/tmp` because DrvFS
paths can reject Unix socket creation. Pip cache also falls back to that temp
tree when DrvFS reports a Windows owner that pip refuses to use as root.

Model weights are intentionally not stored here by default. The default model
store is:

```text
E:\WSL\vLLM\models
```

The default WSL path for that model store is:

```text
/mnt/e/WSL/vLLM/models
```

Use `scripts/bootstrap.sh` from inside WSL to create or repair the project-owned
vLLM environment under this directory.
