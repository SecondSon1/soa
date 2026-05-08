FROM gradle:jdk25 AS build
WORKDIR /app
COPY build.gradle.kts settings.gradle.kts ./
COPY src/main/avro/ src/main/avro/
RUN gradle dependencies --no-daemon --quiet || gradle dependencies --no-daemon --quiet
COPY src/ src/
RUN gradle installDist --no-daemon

FROM eclipse-temurin:25-jre AS runtime
WORKDIR /app
COPY --from=build /app/build/install/consumer/ ./
CMD ["./bin/consumer"]
