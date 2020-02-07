FROM rustembedded/cross:armv7-unknown-linux-gnueabihf

RUN sed -i "s/^deb /deb \[arch=$(dpkg --print-architecture)] /" /etc/apt/sources.list

RUN for SUFFIX in "" "-updates" "-security"; do \
  echo "deb [arch=armhf] http://ports.ubuntu.com/ubuntu-ports/ xenial${SUFFIX} main restricted universe multiverse" \
  >> /etc/apt/sources.list.d/armhf.list; \
done

RUN dpkg --add-architecture armhf && \
    apt-get update && \
    apt-get install -y libv4l-dev:armhf
