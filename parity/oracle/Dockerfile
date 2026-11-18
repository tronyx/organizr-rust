# syntax=docker/dockerfile:1

# The reference for the parity tests: the PHP application itself, at the exact commit the
# Rust application is checked against, behind nginx and PHP-FPM like the official container.
# It is a test tool. It is not part of the product and is never published.

# The PHP version of a current official Organizr container.
ARG PHP_IMAGE=php:8.5-fpm@sha256:baf3ea13a8207fa18e1011bad1016e8b5e59fc974c6966d0efa6f36a16f875f0

FROM ${PHP_IMAGE}

# Organizr 2.1.5000: causefx/Organizr, branch v2-develop.
ARG ORGANIZR_COMMIT=2c1d41997f6ed61b5a760412efef8a7386a68e62

# zip is the one extension Organizr needs that the PHP image does not have.
# The package versions are not pinned: the base image is pinned by digest, and Debian
# removes superseded versions from its mirrors.
# hadolint ignore=DL3008
RUN apt-get update \
    && apt-get install -y --no-install-recommends git libzip-dev nginx \
    && docker-php-ext-install zip \
    && mv "${PHP_INI_DIR}/php.ini-production" "${PHP_INI_DIR}/php.ini" \
    && rm -rf /var/lib/apt/lists/* /etc/nginx/sites-enabled/default

WORKDIR /var/www/html

# Fetching by commit id makes the source exact: git verifies the content against the id.
# The checkout happens in a directory that root owns, because git refuses to work in the
# web root, which belongs to www-data.
RUN git init -q /tmp/organizr \
    && git -C /tmp/organizr fetch -q --depth 1 https://github.com/causefx/Organizr.git "${ORGANIZR_COMMIT}" \
    && git -C /tmp/organizr checkout -q FETCH_HEAD \
    && rm -rf /tmp/organizr/.git \
    && cp -a /tmp/organizr/. /var/www/html/ \
    && rm -rf /tmp/organizr \
    && chown -R www-data:www-data /var/www/html

COPY nginx.conf /etc/nginx/conf.d/organizr.conf

EXPOSE 80

# Both processes belong to one test container; nginx stays in the foreground.
CMD ["sh", "-c", "php-fpm -D && exec nginx -g 'daemon off;'"]
