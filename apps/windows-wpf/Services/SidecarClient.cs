using System.Diagnostics;
using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization;
using AgentCafe.Windows.Models;

namespace AgentCafe.Windows.Services;

public sealed class SidecarClient
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
    };
    private static readonly JsonSerializerOptions PrettyJsonOptions = new()
    {
        WriteIndented = true
    };

    private readonly string _sidecarPath;
    private int _nextId;

    public SidecarClient(string? sidecarPath = null)
    {
        _sidecarPath = sidecarPath
            ?? Environment.GetEnvironmentVariable("AGENTCAFE_SIDECAR")
            ?? ResolveDefaultSidecarPath();
    }

    public async Task<SidecarRunResult<DiagnosticReport>> RunDoctorAsync(
        CancellationToken cancellationToken)
    {
        if (Environment.GetEnvironmentVariable("AGENTCAFE_UI_FIXTURE") is { Length: > 0 } fixture)
        {
            return await LoadFixtureAsync(fixture, cancellationToken);
        }

        if (!File.Exists(_sidecarPath))
        {
            return SidecarRunResult<DiagnosticReport>.Failure(
                "sidecar_missing",
                $"Sidecar not found at {_sidecarPath}.");
        }

        using var process = StartSidecar();
        try
        {
            await SendAsync(process, "ipc.handshake", new
            {
                protocol_version = "1.0",
                ui_name = "agentcafe-windows-wpf",
                ui_version = "0.1.0",
                ui_platform = "windows",
                ui_capabilities = Array.Empty<string>(),
                nonce = Guid.NewGuid().ToString("N")
            }, cancellationToken);

            var handshake = await ReadResponseAsync<JsonElement>(process, cancellationToken);
            if (!handshake.IsSuccess)
            {
                return SidecarRunResult<DiagnosticReport>.Failure(
                    handshake.ErrorCode ?? "handshake_failed",
                    handshake.ErrorMessage ?? "Handshake failed.");
            }

            await SendAsync(process, "doctor.run", new { }, cancellationToken);
            var doctor = await ReadDoctorResponseAsync(process, cancellationToken);
            if (!doctor.IsSuccess || doctor.Result is null)
            {
                return SidecarRunResult<DiagnosticReport>.Failure(
                    doctor.ErrorCode ?? "doctor_failed",
                    doctor.ErrorMessage ?? "doctor.run failed.");
            }

            return SidecarRunResult<DiagnosticReport>.Success(doctor.Result, doctor.PrettyJson ?? "");
        }
        catch (OperationCanceledException)
        {
            TryKill(process);
            return SidecarRunResult<DiagnosticReport>.Failure(
                "timeout",
                "Sidecar request timed out.");
        }
        catch (Exception ex) when (ex is IOException or InvalidOperationException)
        {
            return SidecarRunResult<DiagnosticReport>.Failure("sidecar_crash", ex.Message);
        }
        finally
        {
            if (!process.HasExited)
            {
                process.StandardInput.Close();
            }
        }
    }

    private Process StartSidecar()
    {
        var process = new Process
        {
            StartInfo = new ProcessStartInfo
            {
                FileName = _sidecarPath,
                RedirectStandardInput = true,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            },
            EnableRaisingEvents = true
        };
        if (!process.Start())
        {
            throw new InvalidOperationException("Unable to start sidecar.");
        }
        return process;
    }

    private async Task SendAsync(
        Process process,
        string method,
        object parameters,
        CancellationToken cancellationToken)
    {
        var id = Interlocked.Increment(ref _nextId).ToString();
        var request = new
        {
            jsonrpc = "2.0",
            id,
            method,
            @params = parameters
        };
        var line = JsonSerializer.Serialize(request, JsonOptions);
        await process.StandardInput.WriteLineAsync(line.AsMemory(), cancellationToken);
        await process.StandardInput.FlushAsync(cancellationToken);
    }

    private static async Task<RpcReadResult<T>> ReadResponseAsync<T>(
        Process process,
        CancellationToken cancellationToken)
    {
        var line = await process.StandardOutput.ReadLineAsync(cancellationToken);
        if (string.IsNullOrWhiteSpace(line))
        {
            return RpcReadResult<T>.Failure("sidecar_crash", "Sidecar closed stdout.");
        }

        var envelope = JsonSerializer.Deserialize<RpcEnvelope<T>>(line, JsonOptions);
        if (envelope?.Error is not null)
        {
            return RpcReadResult<T>.Failure(
                envelope.Error.Data?.Code ?? "sidecar_error",
                envelope.Error.Message);
        }
        return RpcReadResult<T>.Success(envelope!.Result);
    }

    private static async Task<RpcReadResult<DiagnosticReport>> ReadDoctorResponseAsync(
        Process process,
        CancellationToken cancellationToken)
    {
        var line = await process.StandardOutput.ReadLineAsync(cancellationToken);
        if (string.IsNullOrWhiteSpace(line))
        {
            return RpcReadResult<DiagnosticReport>.Failure("sidecar_crash", "Sidecar closed stdout.");
        }

        var envelope = JsonSerializer.Deserialize<RpcEnvelope<JsonElement>>(line, JsonOptions);
        if (envelope?.Error is not null)
        {
            return RpcReadResult<DiagnosticReport>.Failure(
                envelope.Error.Data?.Code ?? "sidecar_error",
                envelope.Error.Message);
        }

        var resultJson = envelope!.Result.GetRawText();
        var report = JsonSerializer.Deserialize<DiagnosticReport>(resultJson, JsonOptions);
        var prettyJson = JsonNode.Parse(resultJson)?.ToJsonString(PrettyJsonOptions) ?? resultJson;
        return RpcReadResult<DiagnosticReport>.Success(report, prettyJson);
    }

    private static async Task<SidecarRunResult<DiagnosticReport>> LoadFixtureAsync(
        string fixture,
        CancellationToken cancellationToken)
    {
        var json = await File.ReadAllTextAsync(fixture, cancellationToken);
        var report = JsonSerializer.Deserialize<DiagnosticReport>(json, JsonOptions);
        var prettyJson = JsonNode.Parse(json)?.ToJsonString(PrettyJsonOptions) ?? json;
        return report is null
            ? SidecarRunResult<DiagnosticReport>.Failure("fixture_invalid", "Fixture did not decode.")
            : SidecarRunResult<DiagnosticReport>.Success(report, prettyJson);
    }

    private static string ResolveDefaultSidecarPath()
    {
        var root = FindRepositoryRoot(AppContext.BaseDirectory);
        var fileName = OperatingSystem.IsWindows()
            ? "agentcafe-sidecar.exe"
            : "agentcafe-sidecar";
        return Path.Combine(root, "target", "debug", fileName);
    }

    private static string FindRepositoryRoot(string start)
    {
        var directory = new DirectoryInfo(start);
        while (directory is not null)
        {
            if (File.Exists(Path.Combine(directory.FullName, "Cargo.toml"))
                && Directory.Exists(Path.Combine(directory.FullName, "core")))
            {
                return directory.FullName;
            }
            directory = directory.Parent;
        }

        return Path.GetFullPath(Path.Combine(AppContext.BaseDirectory, "..", "..", "..", ".."));
    }

    private static void TryKill(Process process)
    {
        try
        {
            if (!process.HasExited)
            {
                process.Kill(entireProcessTree: true);
            }
        }
        catch
        {
            // Best-effort cleanup after timeout or crash.
        }
    }

    private sealed record RpcEnvelope<T>(
        [property: JsonPropertyName("result")] T? Result,
        [property: JsonPropertyName("error")] RpcError? Error
    );

    private sealed record RpcError(
        [property: JsonPropertyName("code")] int Code,
        [property: JsonPropertyName("message")] string Message,
        [property: JsonPropertyName("data")] RpcErrorData? Data
    );

    private sealed record RpcErrorData(
        [property: JsonPropertyName("code")] string? Code,
        [property: JsonPropertyName("stage")] string? Stage
    );

    private sealed record RpcReadResult<T>(
        bool IsSuccess,
        T? Result,
        string? PrettyJson,
        string? ErrorCode,
        string? ErrorMessage)
    {
        public static RpcReadResult<T> Success(T? result, string? prettyJson = null) =>
            new(true, result, prettyJson, null, null);
        public static RpcReadResult<T> Failure(string errorCode, string errorMessage) =>
            new(false, default, null, errorCode, errorMessage);
    }
}

public sealed record SidecarRunResult<T>(
    bool IsSuccess,
    T? Report,
    string? PrettyJson,
    string? ErrorCode,
    string? ErrorMessage)
{
    public static SidecarRunResult<T> Success(T report, string prettyJson) =>
        new(true, report, prettyJson, null, null);
    public static SidecarRunResult<T> Failure(string errorCode, string errorMessage) =>
        new(false, default, null, errorCode, errorMessage);
}
