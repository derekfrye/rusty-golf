# Variables
APP_NAME = rusty-golf
IMAGE_NAME = djf/$(APP_NAME)
SQLX_MIDDLEWARE_SRC = ../sql-middleware
SQLX_MIDDLEWARE_DEST = sql-middleware
USERNM = `id -un 1000`
BUILD_CONTEXT = .

# Default target
.PHONY: all
all: build

# Copy sql-middleware into the build context
$(SQLX_MIDDLEWARE_DEST):
	@echo "Copying sql-middleware into build context..."
	rsync -a --exclude target/ $(SQLX_MIDDLEWARE_SRC)/ $(SQLX_MIDDLEWARE_DEST)/

# Build the Docker image
.PHONY: build
build: $(SQLX_MIDDLEWARE_DEST)
	@echo "Building Docker image..."
	podman build -t $(IMAGE_NAME) $(BUILD_CONTEXT) --build-arg USERNAME=$(USERNM)

# Clean up the build context
.PHONY: clean
clean:
	@echo "Cleaning up build context..."
	rm -rf $(SQLX_MIDDLEWARE_DEST)

# Rebuild the image
.PHONY: rebuild
rebuild: clean all
