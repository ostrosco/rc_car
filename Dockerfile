FROM rustembedded/cross:armv7-unknown-linux-gnueabihf

RUN sed -i "s/^deb /deb \[arch=$(dpkg --print-architecture)] /" /etc/apt/sources.list

RUN for SUFFIX in "" "-updates" "-security"; do \
  echo "deb [arch=armhf] http://ports.ubuntu.com/ubuntu-ports/ xenial${SUFFIX} main restricted universe multiverse" \
  >> /etc/apt/sources.list.d/armhf.list; \
done

RUN dpkg --add-architecture armhf && \
    apt-get update && \
    apt install -y wget cmake python3

RUN wget -O opencv.tar.gz https://github.com/opencv/opencv/archive/4.2.0.tar.gz && \
    tar xvf opencv.tar.gz
RUN cd opencv-4.2.0 && \
    mkdir build && \
    cd build && \
    cmake -DCMAKE_TOOLCHAIN_FILE=../platforms/linux/arm-gnueabi.toolchain.cmake \
        -DOPENCV_GENERATE_PKGCONFIG=on \
        -DCMAKE_INSTALL_PREFIX=/usr/local .. && \
    make -j4 && \
    make install
