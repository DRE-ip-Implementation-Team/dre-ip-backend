FROM rust

EXPOSE 8000

WORKDIR /usr/src/myapp

COPY . .

ENV ROCKET_ADDRESS=0.0.0.0

CMD ["cargo", "run"]
