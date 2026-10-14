// Product-owned, read-only typed WinGet query. Never call installation/download APIs.
using Microsoft.Management.Deployment;
using System.Text;
using System.Text.Json;
using System.Runtime.InteropServices;
using WinRT;

internal record UpdateRow(string display_name, string package_id, string installed_version, string? target_version, bool update_available);
internal record QueryResult(int schema, string status, string? code, List<UpdateRow> rows);
internal static class Program
{
    private static string Phase="ACTIVATE";
    internal const string SourceId = "Microsoft.Winget.Source_8wekyb3d8bbwe";
    internal const string SourceUrl = "https://cdn.winget.microsoft.com/cache";
    private static bool Official(PackageCatalogInfo? info) => info != null && info.Id == SourceId && info.Name == "winget" && info.Type == "Microsoft.PreIndexed.Package" && info.Argument == SourceUrl;
    private static bool TextValid(string? s) => !string.IsNullOrWhiteSpace(s) && s.Length <= 1024 && !s.Any(char.IsControl);
    private static QueryResult Failure(string code) => new(1, "ERROR", code, []);
    // Registered Microsoft out-of-process COM classes, as in Microsoft's factory.
    // No activation of discovered paths or bundled arbitrary DLLs.
    [DllImport("ole32.dll", ExactSpelling=true)]
    private static extern int CoCreateInstance(ref Guid clsid, IntPtr outer, uint context, ref Guid iid, out IntPtr result);
    [DllImport("ole32.dll", ExactSpelling=true)]
    private static extern int CoInitializeEx(IntPtr reserved,uint mode);
    [DllImport("ole32.dll", ExactSpelling=true)]
    private static extern void CoUninitialize();
    private static T Activate<T>(string clsid,string iid) where T : new()
    {
        var classId=Guid.Parse(clsid);var interfaceId=Guid.Parse(iid);
        // CLSCTX_LOCAL_SERVER only: never activate registry in-process DLLs/remote servers.
        Marshal.ThrowExceptionForHR(CoCreateInstance(ref classId,IntPtr.Zero,4,ref interfaceId,out var pointer));
        try {return MarshalInterface<T>.FromAbi(pointer);}
        finally {Marshal.Release(pointer);}
    }
    private static QueryResult Query()
    {
        if(Environment.GetEnvironmentVariable("SAFEPATCH_WINGET_COM_VERIFIED")!="1") return Failure("WINGET_UPDATE_COM_UNTRUSTED");
        var manager = Activate<PackageManager>("C53A4F16-787E-42A4-B304-29EFFB4BF597","B375E3B9-F2E0-5C93-87A7-B67497F7E593");
        Phase="SOURCE";
        var source = manager.GetPackageCatalogByName("winget");
        if (source == null || !Official(source.Info)) return Failure("WINGET_UPDATE_SOURCE_UNTRUSTED");
        Phase="AGREEMENTS";
        source.AcceptSourceAgreements = false;
        // No explicit source refresh, agreement acceptance, source changes or REST lookups.
        source.PackageCatalogBackgroundUpdateInterval = TimeSpan.Zero;
        Phase="COMPOSITE_ACTIVATE";
        var options = Activate<CreateCompositePackageCatalogOptions>("526534B8-7E46-47C8-8416-B1685C327D37","21ABAA76-089D-51C5-A745-C85EEFE70116");
        options.Catalogs.Add(source);
        options.CompositeSearchBehavior = CompositeSearchBehavior.LocalCatalogs;
        Phase="CONNECT";
        var connected = manager.CreateCompositePackageCatalog(options).Connect();
        if (connected.Status != ConnectResultStatus.Ok) return Failure("WINGET_UPDATE_CONNECT_FAILED");
        Phase="FIND_ACTIVATE";
        var find = Activate<FindPackagesOptions>("572DED96-9C60-4526-8F92-EE7D91D38C1A","A5270EDD-7DA7-57A3-BACE-F2593553561F");
        find.ResultLimit=20000;
        Phase="SEARCH";
        var result = connected.PackageCatalog.FindPackages(find);
        if (result.Status != FindPackagesResultStatus.Ok) return Failure("WINGET_UPDATE_SEARCH_FAILED");
        if (result.WasLimitExceeded || result.Matches.Count > 20000) return Failure("WINGET_UPDATE_LIMIT");
        var rows = new List<UpdateRow>();
        for (var i = 0; i < result.Matches.Count; i++)
        {
            Phase="PACKAGE";
            var package = result.Matches[i].CatalogPackage;
            var installed = package.InstalledVersion;
            var available = package.DefaultInstallVersion;
            // Uncorrelated local products do not establish update status.
            if (installed == null || available == null || !Official(available.PackageCatalog.Info)) continue;
            if (!TextValid(package.Id) || !TextValid(installed.Version) || !TextValid(available.Version)) continue;
            if (package.Id != available.Id) continue;
            var newer = package.IsUpdateAvailable;
            if (newer && available.CompareToVersion(installed.Version) != CompareResult.Greater) continue;
            var name = TextValid(installed.DisplayName) ? installed.DisplayName : package.Id;
            rows.Add(new(name, package.Id, installed.Version, newer ? available.Version : null, newer));
        }
        return new(1, "COMPLETE", null, rows);
    }
    [STAThread]
    private static int Main(string[] args)
    {
        Console.OutputEncoding = new UTF8Encoding(false);
        QueryResult result;
        if (args.Length != 1 || args[0] != "query") result = Failure("WINGET_UPDATE_INVALID_MODE");
        else
        {
            var initialized=false;
            try {Marshal.ThrowExceptionForHR(CoInitializeEx(IntPtr.Zero,2));initialized=true;result=Query();}
            catch { result = Failure("WINGET_UPDATE_"+Phase+"_FAILED"); }
            finally {if(initialized)CoUninitialize();}
        }
        // All names/versions are escaped JSON data; no raw HRESULT/exception/path output.
        var json = JsonSerializer.Serialize(result);
        if (Encoding.UTF8.GetByteCount(json) > 4 * 1024 * 1024) json = JsonSerializer.Serialize(Failure("WINGET_UPDATE_LIMIT"));
        Console.WriteLine(json);
        return 0;
    }
}
