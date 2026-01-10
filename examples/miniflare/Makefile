.PHONY: all clean helper-up helper-down

# --- Config ---
WORKVOL      := miniflare_working
HELPER_NAME  := miniflare-work-helper
HELPER_IMAGE := alpine
REPO_ROOT    := $(abspath ../..)

all: helper-up
	podman build -t djf/m-golf-srvless -f Dockerfile .

	# Clean volume contents safely (no risky .* glob)
	podman exec $(HELPER_NAME) sh -c 'find /miniflare_work -mindepth 1 -maxdepth 1 -exec rm -rf {} +'

	# Copy project pieces into the mounted volume via the helper container
	podman cp $(REPO_ROOT)/core      $(HELPER_NAME):/miniflare_work/
	podman cp $(REPO_ROOT)/serverless $(HELPER_NAME):/miniflare_work/
	podman cp $(REPO_ROOT)/static    $(HELPER_NAME):/miniflare_work/

	# Ensure bin/ exists, then copy worker-build
	podman exec $(HELPER_NAME) sh -c "mkdir -p /miniflare_work/bin"
	podman cp ./worker-build $(HELPER_NAME):/miniflare_work/bin/

	# Tear down helper after successful build/copy
	$(MAKE) helper-down

# Bring up an ephemeral helper container with the named volume mounted.
helper-up:
	# Create the volume if it doesn't exist (no-op if it already does)
	podman volume inspect $(WORKVOL) >/dev/null 2>&1 || podman volume create $(WORKVOL) >/dev/null

	# Remove any stale helper container from a previous run
	-podman rm -f $(HELPER_NAME) >/dev/null 2>&1 || true

	# Start a tiny container that just stays up while we exec/cp into it
	podman run -d --name $(HELPER_NAME) \
		-v $(WORKVOL):/miniflare_work \
		$(HELPER_IMAGE) sh -c "sleep infinity" >/dev/null

helper-down:
	-podman rm -f $(HELPER_NAME) >/dev/null 2>&1 || true

clean: helper-down
	@true
