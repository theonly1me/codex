#define UNICODE
#define _UNICODE
#include <windows.h>
#include <aclapi.h>
#include <sddl.h>
#include <stdio.h>
#include <stdlib.h>

#define REQUIRE(call) do { if (!(call)) { fprintf(stderr, "%s failed: %lu\n", #call, GetLastError()); exit(1); } } while (0)

static void set_acl(const wchar_t *path, const wchar_t *descriptor) {
    PSECURITY_DESCRIPTOR security = NULL;
    REQUIRE(ConvertStringSecurityDescriptorToSecurityDescriptorW(descriptor, SDDL_REVISION_1, &security, NULL));
    REQUIRE(SetFileSecurityW(path, DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION, security));
    LocalFree(security);
}

static BOOL access_allowed(HANDLE token, wchar_t *path, DWORD access) {
    PSECURITY_DESCRIPTOR descriptor = NULL;
    DWORD result = GetNamedSecurityInfoW(path, SE_FILE_OBJECT,
        OWNER_SECURITY_INFORMATION | GROUP_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
        NULL, NULL, NULL, NULL, &descriptor);
    if (result != ERROR_SUCCESS) { fprintf(stderr, "GetNamedSecurityInfoW failed: %lu\n", result); exit(1); }
    GENERIC_MAPPING mapping = { FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_GENERIC_EXECUTE, FILE_ALL_ACCESS };
    BYTE privileges[1024];
    DWORD privilege_size = sizeof(privileges), granted = 0;
    BOOL allowed = FALSE;
    REQUIRE(AccessCheck(descriptor, token, access, &mapping,
        (PPRIVILEGE_SET)privileges, &privilege_size, &granted, &allowed));
    LocalFree(descriptor);
    return allowed;
}

static void create_file(const wchar_t *path) {
    HANDLE file = CreateFileW(path, GENERIC_WRITE, 0, NULL, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, NULL);
    REQUIRE(file != INVALID_HANDLE_VALUE);
    REQUIRE(CloseHandle(file));
}

int wmain(void) {
    HANDLE base = NULL, restricted = NULL, impersonation = NULL;
    REQUIRE(OpenProcessToken(GetCurrentProcess(), TOKEN_ALL_ACCESS, &base));
    DWORD user_size = 0, group_size = 0;
    GetTokenInformation(base, TokenUser, NULL, 0, &user_size);
    GetTokenInformation(base, TokenGroups, NULL, 0, &group_size);
    TOKEN_USER *user = malloc(user_size);
    TOKEN_GROUPS *groups = malloc(group_size);
    REQUIRE(user && groups);
    REQUIRE(GetTokenInformation(base, TokenUser, user, user_size, &user_size));
    REQUIRE(GetTokenInformation(base, TokenGroups, groups, group_size, &group_size));
    wchar_t *user_sid = NULL;
    REQUIRE(ConvertSidToStringSidW(user->User.Sid, &user_sid));
    PSID capability = NULL, everyone = NULL, logon = NULL;
    const wchar_t *capability_text = L"S-1-5-21-1976432-497324-975342-1123";
    REQUIRE(ConvertStringSidToSidW(capability_text, &capability));
    REQUIRE(ConvertStringSidToSidW(L"S-1-1-0", &everyone));
    for (DWORD index = 0; index < groups->GroupCount; ++index) {
        if ((groups->Groups[index].Attributes & SE_GROUP_LOGON_ID) == SE_GROUP_LOGON_ID)
            logon = groups->Groups[index].Sid;
    }
    REQUIRE(logon != NULL);
    SID_AND_ATTRIBUTES restricting[] = { {capability, 0}, {logon, 0}, {everyone, 0} };
    REQUIRE(CreateRestrictedToken(base, DISABLE_MAX_PRIVILEGE | LUA_TOKEN | WRITE_RESTRICTED,
        0, NULL, 0, NULL, 3, restricting, &restricted));
    REQUIRE(DuplicateToken(restricted, SecurityImpersonation, &impersonation));
    wchar_t temporary[MAX_PATH], root[MAX_PATH], workspace[MAX_PATH], outside[MAX_PATH];
    wchar_t outside_file[MAX_PATH], workspace_file[MAX_PATH], git_directory[MAX_PATH];
    REQUIRE(GetTempPathW(MAX_PATH, temporary) != 0);
    REQUIRE(GetTempFileNameW(temporary, L"cdx", 0, root) != 0);
    REQUIRE(DeleteFileW(root));
    REQUIRE(CreateDirectoryW(root, NULL));
    swprintf_s(workspace, MAX_PATH, L"%ls\\workspace", root);
    swprintf_s(outside, MAX_PATH, L"%ls\\outside", root);
    swprintf_s(outside_file, MAX_PATH, L"%ls\\outside.txt", outside);
    swprintf_s(workspace_file, MAX_PATH, L"%ls\\workspace.txt", workspace);
    swprintf_s(git_directory, MAX_PATH, L"%ls\\.git", workspace);
    REQUIRE(CreateDirectoryW(workspace, NULL));
    REQUIRE(CreateDirectoryW(outside, NULL));
    for (int deny_parent_delete = 0; deny_parent_delete <= 1; ++deny_parent_delete) {
        wchar_t descriptor[2048];
        const wchar_t *deny = deny_parent_delete ? L"(D;;0x40;;;WD)" : L"";
        swprintf_s(descriptor, 2048, L"D:P%ls(A;OICI;FA;;;%ls)", deny, user_sid);
        set_acl(outside, descriptor);
        swprintf_s(descriptor, 2048, L"D:P%ls(A;OICI;FA;;;%ls)(A;OICI;0x1301bf;;;%ls)",
            deny, user_sid, capability_text);
        set_acl(workspace, descriptor);
        create_file(outside_file);
        create_file(workspace_file);
        REQUIRE(CreateDirectoryW(git_directory, NULL));
        swprintf_s(descriptor, 2048, L"D:P(D;;0x130116;;;%ls)(A;;FA;;;%ls)", capability_text, user_sid);
        set_acl(git_directory, descriptor);
        BOOL object_delete = access_allowed(impersonation, outside_file, DELETE);
        BOOL parent_delete = access_allowed(impersonation, outside, FILE_DELETE_CHILD);
        BOOL workspace_delete = access_allowed(impersonation, workspace_file, DELETE);
        REQUIRE(ImpersonateLoggedOnUser(restricted));
        BOOL deleted_outside = DeleteFileW(outside_file);
        DWORD outside_error = deleted_outside ? 0 : GetLastError();
        BOOL deleted_git = RemoveDirectoryW(git_directory);
        DWORD git_error = deleted_git ? 0 : GetLastError();
        BOOL deleted_workspace = DeleteFileW(workspace_file);
        DWORD workspace_error = deleted_workspace ? 0 : GetLastError();
        REQUIRE(RevertToSelf());
        printf("{\"deny_parent_delete\":%d,\"object_delete_allowed\":%d,\"parent_delete_allowed\":%d,"
            "\"workspace_delete_allowed\":%d,\"deleted_outside\":%d,\"outside_error\":%lu,"
            "\"deleted_git\":%d,\"git_error\":%lu,\"deleted_workspace\":%d,\"workspace_error\":%lu}\n",
            deny_parent_delete, object_delete, parent_delete, workspace_delete,
            deleted_outside, outside_error, deleted_git, git_error, deleted_workspace, workspace_error);
        if (!deleted_outside) REQUIRE(DeleteFileW(outside_file));
        if (!deleted_git) REQUIRE(RemoveDirectoryW(git_directory));
        if (!deleted_workspace) REQUIRE(DeleteFileW(workspace_file));
    }
    REQUIRE(RemoveDirectoryW(outside));
    REQUIRE(RemoveDirectoryW(workspace));
    REQUIRE(RemoveDirectoryW(root));
    CloseHandle(impersonation);
    CloseHandle(restricted);
    CloseHandle(base);
    LocalFree(user_sid);
    LocalFree(capability);
    LocalFree(everyone);
    free(groups);
    free(user);
    return 0;
}
