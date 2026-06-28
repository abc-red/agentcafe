using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Windows.Input;
using AgentCafe.Windows.Models;
using AgentCafe.Windows.Services;

namespace AgentCafe.Windows.ViewModels;

public sealed class MainViewModel : INotifyPropertyChanged
{
    private readonly SidecarClient _sidecarClient = new();
    private DiagnosticReport? _report;
    private SectionViewModel? _selectedSection;
    private string _connectionState = "Not connected";
    private string _statusMessage = "Ready to run a read-only diagnostic scan.";
    private bool _isLoading;

    public MainViewModel()
    {
        Sections = new ObservableCollection<SectionViewModel>
        {
            new("总览", "Overview"),
            new("Codex助手", "Codex"),
            new("Claude助手", "Claude"),
            new("MCP", "MCP"),
            new("Plugins", "Plugins"),
            new("Skills", "Skills"),
            new("风险", "Risks"),
            new("备份", "Backups")
        };
        SelectedSection = Sections[0];
        RefreshCommand = new AsyncCommand(RefreshAsync, () => !IsLoading);
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public ObservableCollection<SectionViewModel> Sections { get; }

    public ICommand RefreshCommand { get; }

    public SectionViewModel? SelectedSection
    {
        get => _selectedSection;
        set => SetField(ref _selectedSection, value);
    }

    public DiagnosticReport? Report
    {
        get => _report;
        private set
        {
            if (SetField(ref _report, value))
            {
                OnPropertyChanged(nameof(HasReport));
                OnPropertyChanged(nameof(GeneratedAtDisplay));
                OnPropertyChanged(nameof(Runtimes));
                OnPropertyChanged(nameof(CodexRuntimes));
                OnPropertyChanged(nameof(ClaudeRuntimes));
                OnPropertyChanged(nameof(ConfigSources));
                OnPropertyChanged(nameof(Plugins));
                OnPropertyChanged(nameof(Skills));
                OnPropertyChanged(nameof(McpServers));
                OnPropertyChanged(nameof(Risks));
                OnPropertyChanged(nameof(BackupRows));
                OnPropertyChanged(nameof(SummaryCards));
            }
        }
    }

    public bool HasReport => Report is not null;

    public string ConnectionState
    {
        get => _connectionState;
        private set => SetField(ref _connectionState, value);
    }

    public string StatusMessage
    {
        get => _statusMessage;
        private set => SetField(ref _statusMessage, value);
    }

    public bool IsLoading
    {
        get => _isLoading;
        private set
        {
            if (SetField(ref _isLoading, value) && RefreshCommand is AsyncCommand command)
            {
                command.RaiseCanExecuteChanged();
            }
        }
    }

    public string GeneratedAtDisplay => Report is null
        ? "No scan yet"
        : Report.GeneratedAt.ToLocalTime().ToString("yyyy-MM-dd HH:mm:ss");

    public IReadOnlyList<RuntimeProfile> Runtimes => Report?.Runtimes ?? Array.Empty<RuntimeProfile>();

    public IReadOnlyList<RuntimeProfile> CodexRuntimes => Runtimes
        .Where(runtime => runtime.Runtime == "codex")
        .ToArray();

    public IReadOnlyList<RuntimeProfile> ClaudeRuntimes => Runtimes
        .Where(runtime => runtime.Runtime == "claude")
        .ToArray();

    public IReadOnlyList<ConfigSource> ConfigSources => Report?.ConfigSources ?? Array.Empty<ConfigSource>();

    public IReadOnlyList<PluginItem> Plugins => Report?.Plugins ?? Array.Empty<PluginItem>();

    public IReadOnlyList<SkillItem> Skills => Report?.Skills ?? Array.Empty<SkillItem>();

    public IReadOnlyList<McpServerItem> McpServers => Report?.McpServers ?? Array.Empty<McpServerItem>();

    public IReadOnlyList<RiskFinding> Risks => Report?.RiskFindings ?? Array.Empty<RiskFinding>();

    public IReadOnlyList<BackupRow> BackupRows => new[]
    {
        new BackupRow("Snapshot list", "尚未启用", "MVP2 draft: backup.list returns feature_not_in_mvp."),
        new BackupRow("Create snapshot", "尚未启用", "MVP2 draft: backup.create is not called by this UI."),
        new BackupRow("Restore snapshot", "尚未启用", "MVP2 draft: backup.restore is not called by this UI.")
    };

    public IReadOnlyList<SummaryCard> SummaryCards
    {
        get
        {
            if (Report is null)
            {
                return Array.Empty<SummaryCard>();
            }

            var summary = Report.Summary;
            return new[]
            {
                new SummaryCard("Overall", summary.OverallStatus),
                new SummaryCard("Runtimes", summary.RuntimeCount.ToString()),
                new SummaryCard("Config sources", summary.ConfigSourceCount.ToString()),
                new SummaryCard("MCP servers", summary.McpServerCount.ToString()),
                new SummaryCard("Plugins", summary.PluginCount.ToString()),
                new SummaryCard("Skills", summary.SkillCount.ToString()),
                new SummaryCard("Risks", TotalRiskCount(summary).ToString()),
                new SummaryCard("Truncated", summary.Truncated ? "Yes" : "No")
            };
        }
    }

    private async Task RefreshAsync()
    {
        IsLoading = true;
        ConnectionState = "Connecting";
        StatusMessage = "Starting sidecar and running doctor.run...";

        using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(30));
        var result = await _sidecarClient.RunDoctorAsync(timeout.Token);
        if (result.IsSuccess && result.Report is not null)
        {
            Report = result.Report;
            ConnectionState = "Connected";
            StatusMessage = $"Read-only diagnostic loaded. Trace: {result.Report.TraceId}";
        }
        else
        {
            ConnectionState = result.ErrorCode ?? "Unavailable";
            StatusMessage = NormalizeError(result.ErrorCode, result.ErrorMessage);
        }

        IsLoading = false;
    }

    private static int TotalRiskCount(DiagnosticSummary summary) =>
        summary.RiskCountBySeverity.Info
        + summary.RiskCountBySeverity.Low
        + summary.RiskCountBySeverity.Medium
        + summary.RiskCountBySeverity.High
        + summary.RiskCountBySeverity.Critical;

    private static string NormalizeError(string? code, string? message) => code switch
    {
        "feature_not_in_mvp" => "尚未启用：此操作属于 MVP2 draft。",
        "sidecar_missing" => "Sidecar missing. Build agentcafe-sidecar or set AGENTCAFE_SIDECAR.",
        "handshake_failed" => "Handshake failed. Check protocol version compatibility.",
        "timeout" => "Sidecar request timed out.",
        "sidecar_crash" => "Sidecar crashed or closed the IPC stream.",
        _ => message ?? "Unable to load diagnostic report."
    };

    private bool SetField<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value))
        {
            return false;
        }

        field = value;
        OnPropertyChanged(propertyName);
        return true;
    }

    private void OnPropertyChanged([CallerMemberName] string? propertyName = null) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
}

public sealed record SectionViewModel(string Title, string Key);

public sealed record SummaryCard(string Label, string Value);

public sealed record BackupRow(string Name, string State, string Detail);

public sealed class AsyncCommand : ICommand
{
    private readonly Func<Task> _execute;
    private readonly Func<bool> _canExecute;

    public AsyncCommand(Func<Task> execute, Func<bool> canExecute)
    {
        _execute = execute;
        _canExecute = canExecute;
    }

    public event EventHandler? CanExecuteChanged;

    public bool CanExecute(object? parameter) => _canExecute();

    public async void Execute(object? parameter) => await _execute();

    public void RaiseCanExecuteChanged() => CanExecuteChanged?.Invoke(this, EventArgs.Empty);
}
