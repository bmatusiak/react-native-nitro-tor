

build docker image
```
docker build -f docker/Dockerfile -t rn-tor-android-builder:ci .
```

run build script from mounted repo
```
docker run --rm -v "$(pwd)":/workspace -w /workspace rn-tor-android-builder:ci /workspace/docker/build.sh
```

If the script is not executable on your host, run it via `bash`:
```
docker run --rm -v "$(pwd)":/workspace -w /workspace rn-tor-android-builder:ci bash /workspace/docker/build.sh
```
