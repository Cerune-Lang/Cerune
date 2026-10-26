pub(super) const SOURCE: &str = r#"
#include <errno.h>
static int cerune_file_configured;
static void cerune_file_startup_failed(const char *phase, const char *code) {
    fprintf(stderr, "cerune: file-v1 operation=connect phase=%s code=%s origin=host\n", phase, code);
    fflush(stderr); _Exit(1);
}
#ifndef CERUNE_FILE_TEST_CONNECTION
#ifdef _WIN32
#include <windows.h>
typedef struct { USHORT length, maximum; WCHAR *buffer; } cerune_nt_string;
typedef struct { ULONG length; HANDLE root; cerune_nt_string *name; ULONG attributes; void *security, *quality; } cerune_nt_attributes;
typedef struct { ULONG_PTR status, information; } cerune_nt_status;
typedef LONG (NTAPI *cerune_nt_create)(HANDLE *, ACCESS_MASK, cerune_nt_attributes *, cerune_nt_status *, LARGE_INTEGER *, ULONG, ULONG, ULONG, ULONG, void *, ULONG);
enum {
    CERUNE_NT_CASE_INSENSITIVE = 0x40,
    CERUNE_NT_DIRECTORY = 1,
    CERUNE_NT_NON_DIRECTORY = 0x40,
    CERUNE_NT_SYNCHRONOUS = 0x20,
    CERUNE_NT_OPEN_REPARSE_POINT = 0x200000,
    CERUNE_NT_OPEN = 1,
    CERUNE_NT_CREATE = 2
};
static HANDLE cerune_file_root;
static const char *cerune_file_nt_open(HANDLE root, WCHAR *name, size_t length, int directory, int create, HANDLE *handle) {
    if (length > USHRT_MAX / sizeof(WCHAR)) return "open-failed";
    cerune_nt_string text = {(USHORT)(length * sizeof(WCHAR)), (USHORT)(length * sizeof(WCHAR)), name};
    cerune_nt_attributes attributes = {sizeof(attributes), root, &text, CERUNE_NT_CASE_INSENSITIVE, NULL, NULL};
    cerune_nt_status status = {0};
    cerune_nt_create open_file = (cerune_nt_create)GetProcAddress(GetModuleHandleW(L"ntdll.dll"), "NtCreateFile");
    if (!open_file) return "open-failed";
    /* Synchronize + read attributes + traverse/read/write; never follow reparse points. */
    ULONG access = SYNCHRONIZE | FILE_READ_ATTRIBUTES | (directory ? FILE_TRAVERSE : create ? FILE_WRITE_DATA : FILE_READ_DATA);
    ULONG options = CERUNE_NT_SYNCHRONOUS | CERUNE_NT_OPEN_REPARSE_POINT |
        (directory ? CERUNE_NT_DIRECTORY : CERUNE_NT_NON_DIRECTORY);
    LONG result = open_file(handle, access, &attributes, &status, NULL, 0,
        FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE, create ? CERUNE_NT_CREATE : CERUNE_NT_OPEN, options, NULL, 0);
    if (result < 0) {
        switch ((ULONG)result) {
            case 0xc0000034: case 0xc000003a: return "not-found";
            case 0xc0000035: return "already-exists";
            case 0xc00000ba: return create ? "already-exists" : "unsupported-target";
            case 0xc0000103: case 0xc0000279: case 0xc000050b: return "unsupported-target";
            default: return "open-failed";
        }
    }
    if (!*handle || *handle == INVALID_HANDLE_VALUE) return "invalid-response";
    return NULL;
}
static const char *cerune_file_kind(HANDLE handle, int directory) {
    FILE_ATTRIBUTE_TAG_INFO info;
    if (GetFileType(handle) != FILE_TYPE_DISK) return "unsupported-target";
    if (!GetFileInformationByHandleEx(handle, FileAttributeTagInfo, &info, sizeof(info))) return "open-failed";
    if ((info.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) ||
        !!(info.FileAttributes & FILE_ATTRIBUTE_DIRECTORY) != directory) return "unsupported-target";
    return NULL;
}
static void cerune_file_connect(const wchar_t *root) {
    DWORD length = GetFullPathNameW(root, 0, NULL, NULL);
    if (!length || length > 32760) cerune_file_startup_failed("open-root", "invalid-root");
    WCHAR *absolute = malloc(((size_t)length + 1) * sizeof(WCHAR));
    WCHAR *name = malloc(((size_t)length + 9) * sizeof(WCHAR));
    if (!absolute || !name) cerune_file_startup_failed("open-root", "allocation-failed");
    DWORD actual = GetFullPathNameW(root, length + 1, absolute, NULL);
    if (!actual || actual > length) cerune_file_startup_failed("open-root", "invalid-root");
    size_t offset = 0, prefix = 4;
    memcpy(name, L"\\??\\", 4 * sizeof(WCHAR));
    if (actual >= 4 && !wcsncmp(absolute, L"\\\\?\\", 4)) { offset = 4; }
    else if (actual >= 4 && !wcsncmp(absolute, L"\\\\.\\", 4)) cerune_file_startup_failed("open-root", "invalid-root");
    else if (actual >= 2 && absolute[0] == L'\\' && absolute[1] == L'\\') {
        memcpy(name + 4, L"UNC\\", 4 * sizeof(WCHAR)); prefix = 8; offset = 2;
    }
    if (prefix == 4 && !(actual - offset >= 3 && absolute[offset+1] == L':' && absolute[offset+2] == L'\\') &&
        !(actual - offset >= 4 && !wcsncmp(absolute + offset, L"UNC\\", 4)))
        cerune_file_startup_failed("open-root", "invalid-root");
    memcpy(name + prefix, absolute + offset, (actual - offset) * sizeof(WCHAR));
    size_t n = prefix + actual - offset;
    while (n > 7 && name[n-1] == L'\\') --n;
    HANDLE handle = NULL; const char *code = cerune_file_nt_open(NULL, name, n, 1, 0, &handle);
    free(name); free(absolute);
    if (!code) { code = cerune_file_kind(handle, 1); if (code) CloseHandle(handle); }
    if (code) cerune_file_startup_failed("open-root", code);
    cerune_file_root = handle; cerune_file_configured = 1;
}
static const char *cerune_file_open(cerune_string name, int create, intptr_t *result) {
    WCHAR wide[256];
    int length = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, (const char *)name.data, (int)name.length, wide, 256);
    if (!length) return "open-failed";
    HANDLE handle = NULL; const char *code = cerune_file_nt_open(cerune_file_root, wide, (size_t)length, 0, create, &handle);
    if (!code && !create) { code = cerune_file_kind(handle, 0); if (code) CloseHandle(handle); }
    if (!code) *result = (intptr_t)handle;
    return code;
}
static int64_t cerune_file_read(intptr_t handle, unsigned char *data, size_t length) {
    DWORD count = 0;
    if (ReadFile((HANDLE)handle, data, (DWORD)length, &count, NULL)) return count;
    if (count) return (int64_t)length + 1;
    if (GetLastError() == ERROR_HANDLE_EOF) return 0;
    errno = EIO; return -1;
}
static int64_t cerune_file_write(intptr_t handle, const unsigned char *data, size_t length) {
    DWORD count = 0;
    if (WriteFile((HANDLE)handle, data, (DWORD)length, &count, NULL)) return count;
    if (count) return (int64_t)length + 1;
    errno = EIO; return -1;
}
static int cerune_file_close(intptr_t handle) { return CloseHandle((HANDLE)handle) ? 0 : -1; }
static void cerune_file_disconnect(void) { if (cerune_file_configured) CloseHandle(cerune_file_root); cerune_file_configured = 0; }
#elif defined(__linux__)
#include <fcntl.h>
#include <unistd.h>
#include <sys/stat.h>
static int cerune_file_root = -1;
static const char *cerune_file_open_error(int error, int create) {
    if (error == ENOENT) return "not-found";
    if (error == EEXIST || (create && error == EISDIR)) return "already-exists";
    if (error == ELOOP || error == ENOTDIR || error == EISDIR) return "unsupported-target";
    return "open-failed";
}
static void cerune_file_connect(const char *root) {
    size_t n = strlen(root); char *path = malloc(n + 1);
    if (!path) cerune_file_startup_failed("open-root", "allocation-failed");
    memcpy(path, root, n + 1); while (n > 1 && path[n-1] == '/') path[--n] = 0;
    int fd = open(path, O_PATH | O_NOFOLLOW | O_CLOEXEC); int open_error = errno; free(path);
    if (fd < 0) cerune_file_startup_failed("open-root", cerune_file_open_error(open_error, 0));
    struct stat info;
    const char *code = fstat(fd, &info) ? "open-failed" : !S_ISDIR(info.st_mode) ? "unsupported-target" : NULL;
    if (code) { close(fd); cerune_file_startup_failed("open-root", code); }
    cerune_file_root = fd; cerune_file_configured = 1;
}
static const char *cerune_file_open(cerune_string name, int create, intptr_t *result) {
    char path[256]; memcpy(path, name.data, name.length); path[name.length] = 0;
    int fd = openat(cerune_file_root, path, create ? O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC : O_PATH | O_NOFOLLOW | O_CLOEXEC, 0666);
    if (fd < 0) return cerune_file_open_error(errno, create);
    if (create) { *result = fd; return NULL; }
    struct stat before, after;
    const char *code = fstat(fd, &before) ? "open-failed" : !S_ISREG(before.st_mode) ? "unsupported-target" : NULL;
    if (code) { close(fd); return code; }
    char pinned[64]; snprintf(pinned, sizeof(pinned), "/proc/self/fd/%d", fd);
    int readable = open(pinned, O_RDONLY | O_CLOEXEC); close(fd);
    if (readable < 0) return "open-failed";
    if (fstat(readable, &after)) code = "open-failed";
    else if (!S_ISREG(after.st_mode) || before.st_dev != after.st_dev || before.st_ino != after.st_ino) code = "invalid-response";
    if (code) { close(readable); return code; }
    *result = readable; return NULL;
}
static int64_t cerune_file_read(intptr_t handle, unsigned char *data, size_t length) { return read((int)handle, data, length); }
static int64_t cerune_file_write(intptr_t handle, const unsigned char *data, size_t length) { return write((int)handle, data, length); }
static int cerune_file_close(intptr_t handle) { return close((int)handle); }
static void cerune_file_disconnect(void) { if (cerune_file_configured) close(cerune_file_root); cerune_file_configured = 0; }
#else
static void cerune_file_connect(const char *root) { (void)root; cerune_file_startup_failed("open-root", "unsupported-platform"); }
static const char *cerune_file_open(cerune_string name, int create, intptr_t *result) { (void)name; (void)create; (void)result; return "unsupported-target"; }
static int64_t cerune_file_read(intptr_t h, unsigned char *p, size_t n) { (void)h; (void)p; (void)n; errno = EIO; return -1; }
static int64_t cerune_file_write(intptr_t h, const unsigned char *p, size_t n) { (void)h; (void)p; (void)n; errno = EIO; return -1; }
static int cerune_file_close(intptr_t h) { (void)h; return -1; }
static void cerune_file_disconnect(void) {}
#endif
#else
/* The comparison harness supplies transfers, not alternate language semantics. */
static const char *cerune_file_open(cerune_string, int, intptr_t *);
static int64_t cerune_file_read(intptr_t, unsigned char *, size_t);
static int64_t cerune_file_write(intptr_t, const unsigned char *, size_t);
static int cerune_file_close(intptr_t);
#endif
"#;
