[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Command = 'status',

    [Parameter(Position = 1)]
    [string]$Id,

    [string]$Impact,

    [string]$Note,

    [string]$ValidationStatus,

    [string]$ValidationNote,

    [string]$Reason,

    [switch]$Json
)

$ErrorActionPreference = 'Stop'

$script:RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$script:StatePath = Join-Path $script:RepoRoot 'docs/context/upstream-sync-state.json'
$script:StateRelativePath = 'docs/context/upstream-sync-state.json'
$script:CustomizationMapPath = Join-Path $script:RepoRoot 'docs/context/customization-map.md'
$script:ScriptRelativePath = 'scripts/fork-sync.ps1'
$script:JsonOutput = [bool]$Json
$script:AllowedCommands = @('status', 'prepare', 'review', 'record', 'bootstrap', 'abandon')

function Stop-InvalidInput {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Message
    )

    throw "INVALID_INPUT: $Message"
}

function Get-UtcTimestamp {
    return [DateTime]::UtcNow.ToString('o')
}

function Stop-Blocked {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Message
    )

    throw "BLOCKED: $Message"
}

function Invoke-Git {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    $output = & git -C $script:RepoRoot @Arguments 2>&1
    $exitCode = $LASTEXITCODE
    $text = (($output | ForEach-Object { $_.ToString() }) -join [Environment]::NewLine).Trim()

    if ($exitCode -ne 0) {
        $commandText = ($Arguments -join ' ')
        if ([string]::IsNullOrWhiteSpace($text)) {
            throw "git $commandText failed with exit code $exitCode."
        }
        throw "git $commandText failed with exit code ${exitCode}: $text"
    }

    return $text
}

function Try-Invoke-Git {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    try {
        return Invoke-Git -Arguments $Arguments
    }
    catch {
        return $null
    }
}

function Get-RefSha {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Reference
    )

    $value = Try-Invoke-Git -Arguments @('rev-parse', '--verify', "$Reference^{commit}")
    if ([string]::IsNullOrWhiteSpace($value)) {
        return $null
    }
    return $value.Trim()
}

function Get-RequiredRefSha {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Reference
    )

    $value = Get-RefSha -Reference $Reference
    if ([string]::IsNullOrWhiteSpace($value)) {
        throw "Required Git reference '$Reference' does not exist."
    }
    return $value
}

function Get-CurrentBranch {
    return (Invoke-Git -Arguments @('branch', '--show-current')).Trim()
}

function Assert-CommitSha {
    param(
        [Parameter(Mandatory = $true)]
        [object]$Value,
        [Parameter(Mandatory = $true)]
        [string]$FieldName
    )

    if ($Value -isnot [string] -or $Value -notmatch '^[0-9a-fA-F]{40}$') {
        throw "State field '$FieldName' is not a full 40-character commit SHA."
    }
    $exists = Try-Invoke-Git -Arguments @('cat-file', '-e', "$Value^{commit}")
    if ($null -eq $exists) {
        throw "State field '$FieldName' does not refer to an available commit: $Value"
    }
}

function Assert-NonEmptyStateField {
    param(
        [Parameter(Mandatory = $true)]
        [object]$Object,
        [Parameter(Mandatory = $true)]
        [string]$FieldName
    )

    if ($null -eq $Object) {
        throw "State field '$FieldName' is missing or empty."
    }
    $propertyName = ($FieldName -split '\.')[-1]
    $property = $Object.PSObject.Properties[$propertyName]
    if ($null -eq $property -or $null -eq $property.Value -or [string]::IsNullOrWhiteSpace([string]$property.Value)) {
        throw "State field '$FieldName' is missing or empty."
    }
}

function Assert-ReviewObject {
    param(
        [Parameter(Mandatory = $true)]
        [object]$Review,
        [Parameter(Mandatory = $true)]
        [string]$FieldName
    )

    if ($null -eq $Review) {
        throw "State review '$FieldName' is null."
    }
    $status = Get-PropertyValue -Object $Review -Name 'status' -Default ''
    if (@('reviewed', 'needs-review', 'blocked') -notcontains $status) {
        throw "State review '$FieldName' has invalid status '$status'."
    }
    $impact = Get-PropertyValue -Object $Review -Name 'impact' -Default ''
    if (@('unaffected', 'adapted', 'upstream-equivalent', 'unknown') -notcontains $impact) {
        throw "State review '$FieldName' has invalid impact '$impact'."
    }
    foreach ($shaField in @('last_reviewed_upstream_sha', 'last_reviewed_custom_next_commit')) {
        $sha = Get-PropertyValue -Object $Review -Name $shaField
        if ($null -ne $sha) {
            Assert-CommitSha -Value $sha -FieldName "reviews.$FieldName.$shaField"
        }
    }
}

function Assert-StateShape {
    param(
        [Parameter(Mandatory = $true)]
        [object]$State
    )

    foreach ($section in @('upstream', 'fork')) {
        $sectionValue = Get-PropertyValue -Object $State -Name $section
        if ($null -eq $sectionValue) {
            throw "State section '$section' is missing."
        }
    }
    foreach ($field in @('repository', 'remote', 'ref')) {
        Assert-NonEmptyStateField -Object (Get-PropertyValue -Object $State -Name 'upstream') -FieldName "upstream.$field"
    }
    foreach ($field in @('upstream_mirror_branch', 'development_branch')) {
        Assert-NonEmptyStateField -Object (Get-PropertyValue -Object $State -Name 'fork') -FieldName "fork.$field"
    }

    $completed = Get-PropertyValue -Object $State -Name 'last_completed_sync'
    if ($null -ne $completed) {
        if ((Get-PropertyValue -Object $completed -Name 'status' -Default '') -ne 'complete') {
            throw 'last_completed_sync.status must be complete.'
        }
        foreach ($field in @('upstream_baseline_sha', 'main_integration_commit', 'custom_next_sync_commit')) {
            Assert-CommitSha -Value (Get-PropertyValue -Object $completed -Name $field) -FieldName "last_completed_sync.$field"
        }
        if (@('base', 'merge', 'fast-forward') -notcontains (Get-PropertyValue -Object $completed -Name 'custom_next_sync_kind' -Default '')) {
            throw 'last_completed_sync.custom_next_sync_kind is invalid.'
        }
        if ((Get-PropertyValue -Object $completed -Name 'validation_status' -Default '') -ne 'passed') {
            throw 'last_completed_sync.validation_status must be passed.'
        }
        Assert-NonEmptyStateField -Object $completed -FieldName 'last_completed_sync.recorded_at'
    }

    $reviews = Get-PropertyValue -Object $State -Name 'customization_reviews'
    if ($null -eq $reviews) {
        throw 'customization_reviews is missing.'
    }
    foreach ($property in $reviews.PSObject.Properties) {
        Assert-ReviewObject -Review $property.Value -FieldName $property.Name
    }

    $pending = Get-PropertyValue -Object $State -Name 'pending_review'
    if ($null -ne $pending) {
        if (@('in_progress', 'bootstrap') -notcontains (Get-PropertyValue -Object $pending -Name 'status' -Default '')) {
            throw 'pending_review.status is invalid.'
        }
        Assert-CommitSha -Value (Get-PropertyValue -Object $pending -Name 'candidate_upstream_baseline_sha') -FieldName 'pending_review.candidate_upstream_baseline_sha'
        Assert-NonEmptyStateField -Object $pending -FieldName 'pending_review.prepared_at'
        $pendingReviews = Get-PropertyValue -Object $pending -Name 'reviews'
        if ($null -eq $pendingReviews) {
            throw 'pending_review.reviews is missing.'
        }
        foreach ($property in $pendingReviews.PSObject.Properties) {
            Assert-ReviewObject -Review $property.Value -FieldName "pending_review.reviews.$($property.Name)"
        }
        $validation = Get-PropertyValue -Object $pending -Name 'validation'
        if ($null -eq $validation) {
            throw 'pending_review.validation is missing.'
        }
        if (@('not-run', 'passed', 'failed', 'blocked') -notcontains (Get-PropertyValue -Object $validation -Name 'status' -Default '')) {
            throw 'pending_review.validation.status is invalid.'
        }
    }
}

function Test-Ancestor {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Ancestor,

        [Parameter(Mandatory = $true)]
        [string]$Descendant
    )

    & git -C $script:RepoRoot merge-base --is-ancestor $Ancestor $Descendant 2>$null
    return ($LASTEXITCODE -eq 0)
}

function Get-MergeBase {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Left,

        [Parameter(Mandatory = $true)]
        [string]$Right
    )

    return (Invoke-Git -Arguments @('merge-base', $Left, $Right)).Trim()
}

function Get-CommitCount {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Range
    )

    $value = (Invoke-Git -Arguments @('rev-list', '--count', $Range)).Trim()
    return [int]$value
}

function Get-ChangedFiles {
    param(
        [string]$FromSha,

        [Parameter(Mandatory = $true)]
        [string]$ToSha
    )

    if ([string]::IsNullOrWhiteSpace($FromSha)) {
        $text = Invoke-Git -Arguments @('show', '--format=', '--name-only', $ToSha)
    }
    else {
        $text = Invoke-Git -Arguments @('diff', '--name-only', "$FromSha..$ToSha")
    }

    if ([string]::IsNullOrWhiteSpace($text)) {
        return @()
    }

    return @($text -split "`r?`n" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
}

function Get-WorktreeChanges {
    $text = Invoke-Git -Arguments @('status', '--porcelain=v1', '--untracked-files=all')
    if ([string]::IsNullOrWhiteSpace($text)) {
        return @()
    }

    $changes = New-Object System.Collections.Generic.List[string]
    foreach ($line in ($text -split "`r?`n")) {
        if ([string]::IsNullOrWhiteSpace($line)) {
            continue
        }

        $match = [regex]::Match($line, '^(?:[ MADRCU?!]{2}|[ MADRCU?!])\s+(.*)$')
        if ($match.Success) {
            $path = $match.Groups[1].Value.Trim()
        }
        else {
            $path = $line.Trim()
        }
        $path = $path -replace '\\', '/'
        if ($path -match ' -> ') {
            $path = ($path -split ' -> ')[-1]
        }
        $changes.Add($path)
    }
    return @($changes)
}

function Get-UnrelatedWorktreeChanges {
    $allowed = @(
        $script:StateRelativePath,
        $script:ScriptRelativePath
    )
    $changes = Get-WorktreeChanges
    return @($changes | Where-Object { $allowed -notcontains $_ })
}

function Assert-NoUnrelatedWorktreeChanges {
    $unrelated = Get-UnrelatedWorktreeChanges
    if ($unrelated.Count -gt 0) {
        Stop-Blocked -Message "Unrelated working-tree changes block this operation: $($unrelated -join ', ')"
    }
}

function Assert-DevelopmentBranch {
    $branch = Get-CurrentBranch
    if ($branch -ne 'custom-next') {
        Stop-Blocked -Message "This command must run on 'custom-next'; current branch is '$branch'."
    }
}

function Assert-CommandAllowed {
    if ($script:AllowedCommands -notcontains $Command) {
        Stop-InvalidInput -Message "Unknown command '$Command'. Allowed commands: $($script:AllowedCommands -join ', ')."
    }
}

function Get-PropertyValue {
    param(
        [object]$Object,
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [object]$Default = $null
    )

    if ($null -eq $Object) {
        return $Default
    }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $Default
    }
    if ($null -eq $property.Value) {
        return $Default
    }
    return $property.Value
}

function Set-PropertyValue {
    param(
        [Parameter(Mandatory = $true)]
        [object]$Object,
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [object]$Value
    )

    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        [void]($Object | Add-Member -MemberType NoteProperty -Name $Name -Value $Value)
    }
    else {
        $property.Value = $Value
    }
}

function New-EmptyObject {
    return [pscustomobject]@{}
}

function New-PendingReview {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CandidateUpstreamSha,
        [string]$PreviousUpstreamSha,
        [Parameter(Mandatory = $true)]
        [string]$PreparedFromCustomNextCommit,
        [ValidateSet('in_progress', 'bootstrap')]
        [string]$Status = 'in_progress'
    )

    return [pscustomobject]@{
        status = $Status
        candidate_upstream_baseline_sha = $CandidateUpstreamSha
        prepared_at = Get-UtcTimestamp
        prepared_from_upstream_sha = $PreviousUpstreamSha
        prepared_from_custom_next_commit = $PreparedFromCustomNextCommit
        main_integration_commit = $null
        custom_next_sync_commit = $null
        custom_next_sync_kind = $null
        reviews = New-EmptyObject
        validation = [pscustomobject]@{
            status = 'not-run'
            note = $null
            validated_at = $null
        }
    }
}

function New-InitialState {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CandidateUpstreamSha,
        [Parameter(Mandatory = $true)]
        [string]$CustomNextSha
    )

    return [pscustomobject]@{
        '$schema' = './upstream-sync-state.schema.json'
        schema_version = 1
        kind = 'fork-upstream-sync-state'
        upstream = [pscustomobject]@{
            repository = 'https://github.com/Dicklesworthstone/pi_agent_rust.git'
            remote = 'upstream'
            ref = 'main'
        }
        fork = [pscustomobject]@{
            upstream_mirror_branch = 'main'
            development_branch = 'custom-next'
        }
        last_completed_sync = $null
        customization_reviews = New-EmptyObject
        pending_review = New-PendingReview `
            -CandidateUpstreamSha $CandidateUpstreamSha `
            -PreviousUpstreamSha $null `
            -PreparedFromCustomNextCommit $CustomNextSha `
            -Status 'bootstrap'
    }
}

function Load-State {
    if (-not (Test-Path -LiteralPath $script:StatePath -PathType Leaf)) {
        return $null
    }

    try {
        $raw = Get-Content -LiteralPath $script:StatePath -Raw -Encoding UTF8
        $state = $raw | ConvertFrom-Json
    }
    catch {
        throw "Unable to parse '$($script:StateRelativePath)': $($_.Exception.Message)"
    }

    if ([int](Get-PropertyValue -Object $state -Name 'schema_version' -Default 0) -ne 1) {
        throw "Unsupported or missing state schema_version in '$($script:StateRelativePath)'."
    }
    if ((Get-PropertyValue -Object $state -Name 'kind' -Default '') -ne 'fork-upstream-sync-state') {
        throw "Invalid state kind in '$($script:StateRelativePath)'."
    }

    Assert-StateShape -State $state
    return $state
}

function Save-State {
    param(
        [Parameter(Mandatory = $true)]
        [object]$State
    )

    $directory = Split-Path -Parent $script:StatePath
    if (-not (Test-Path -LiteralPath $directory -PathType Container)) {
        throw "State directory does not exist: $directory"
    }

    Assert-StateShape -State $State
    $json = $State | ConvertTo-Json -Depth 20
    $json = $json.TrimEnd("`r", "`n")
    $json = $json -replace "`r`n", "`n"
    [void]($json | ConvertFrom-Json)
    $temporaryPath = "$($script:StatePath).tmp.$([guid]::NewGuid().ToString('N'))"

    try {
        [System.IO.File]::WriteAllText(
            $temporaryPath,
            $json,
            [System.Text.UTF8Encoding]::new($false)
        )

        if (Test-Path -LiteralPath $script:StatePath -PathType Leaf) {
            $backupPath = "$($script:StatePath).bak.$([guid]::NewGuid().ToString('N'))"
            [System.IO.File]::Replace($temporaryPath, $script:StatePath, $backupPath, $true)
            try {
                [System.IO.File]::Delete($backupPath)
            }
            catch {
                Write-Warning "State replacement succeeded, but backup cleanup failed: $backupPath"
            }
        }
        else {
            [System.IO.File]::Move($temporaryPath, $script:StatePath)
        }
    }
    catch {
        throw "Atomic state write failed. Temporary file was preserved at '$temporaryPath': $($_.Exception.Message)"
    }
}

function Get-ActiveCustomizationIds {
    if (-not (Test-Path -LiteralPath $script:CustomizationMapPath -PathType Leaf)) {
        return @()
    }

    $raw = Get-Content -LiteralPath $script:CustomizationMapPath -Raw -Encoding UTF8
    $matches = [regex]::Matches($raw, '(?ms)^###\s+(FORK-[A-Z0-9-]+).*?(?=^###\s+|\z)')
    $ids = New-Object System.Collections.Generic.List[string]
    foreach ($match in $matches) {
        if ($match.Value -match '- \*\*状态：\*\*\s+`active`') {
            $ids.Add($match.Groups[1].Value)
        }
    }
    return @($ids)
}

function Get-ReviewValue {
    param(
        [object]$Reviews,
        [Parameter(Mandatory = $true)]
        [string]$Id
    )

    if ($null -eq $Reviews) {
        return $null
    }
    $property = $Reviews.PSObject.Properties[$Id]
    if ($null -eq $property) {
        return $null
    }
    return $property.Value
}

function Set-ReviewValue {
    param(
        [Parameter(Mandatory = $true)]
        [object]$Reviews,
        [Parameter(Mandatory = $true)]
        [string]$Id,
        [Parameter(Mandatory = $true)]
        [object]$Value
    )

    Set-PropertyValue -Object $Reviews -Name $Id -Value $Value
}

function Get-SyncKind {
    param(
        [Parameter(Mandatory = $true)]
        [string]$MainSha,
        [Parameter(Mandatory = $true)]
        [string]$CustomNextSha
    )

    if ($MainSha -eq $CustomNextSha) {
        return 'fast-forward'
    }
    if (-not (Test-Ancestor -Ancestor $MainSha -Descendant $CustomNextSha)) {
        return 'unknown'
    }

    $mergeLines = Invoke-Git -Arguments @(
        'log', '--merges', '--format=%H %P', "$MainSha..$CustomNextSha"
    )
    foreach ($line in ($mergeLines -split "`r?`n")) {
        $parts = @($line -split '\s+')
        if ($parts.Count -ge 3 -and ($parts[1..($parts.Count - 1)] -contains $MainSha)) {
            return 'merge'
        }
    }
    return 'unknown'
}

function Get-SyncPosition {
    param(
        [Parameter(Mandatory = $true)]
        [string]$MainSha,
        [Parameter(Mandatory = $true)]
        [string]$CustomNextSha
    )

    if (-not (Test-Ancestor -Ancestor $MainSha -Descendant $CustomNextSha)) {
        throw "custom-next does not contain main; upstream synchronization is incomplete."
    }
    return Get-MergeBase -Left $MainSha -Right $CustomNextSha
}

function New-Result {
    param(
        [Parameter(Mandatory = $true)]
        [string]$CommandName,
        [Parameter(Mandatory = $true)]
        [string]$Status,
        [int]$ExitCode = 0,
        [object]$Data = $null,
        [string]$Message = ''
    )

    return [pscustomobject]@{
        ok = ($ExitCode -eq 0 -or $ExitCode -eq 10)
        command = $CommandName
        status = $Status
        exit_code = $ExitCode
        message = $Message
        data = $Data
    }
}

function Get-StatusResult {
    $stateError = $null
    try {
        $state = Load-State
    }
    catch {
        $state = $null
        $stateError = $_.Exception.Message
    }
    $branch = Get-CurrentBranch
    $changes = Get-WorktreeChanges
    $unrelated = Get-UnrelatedWorktreeChanges
    $upstreamSha = Get-RefSha -Reference 'upstream/main'
    $mainSha = Get-RefSha -Reference 'main'
    $customNextSha = Get-RefSha -Reference 'custom-next'

    if ($null -eq $state) {
        $status = if ($null -ne $stateError) { 'INVALID' } else { 'UNINITIALIZED' }
        $message = if ($null -ne $stateError) { $stateError } else { 'State file has not been created. Run bootstrap on custom-next.' }
        return New-Result -CommandName 'status' -Status $status -ExitCode $(if ($null -ne $stateError) { 2 } else { 10 }) -Message $message -Data ([pscustomobject]@{
            current_branch = $branch
            working_tree_changes = @($changes)
            unrelated_worktree_changes = @($unrelated)
            upstream_main_sha = $upstreamSha
            main_sha = $mainSha
            custom_next_sha = $customNextSha
            state_path = $script:StateRelativePath
        })
    }

    $completed = Get-PropertyValue -Object $state -Name 'last_completed_sync'
    $pending = Get-PropertyValue -Object $state -Name 'pending_review'
    $completedSha = Get-PropertyValue -Object $completed -Name 'upstream_baseline_sha'
    $pendingSha = Get-PropertyValue -Object $pending -Name 'candidate_upstream_baseline_sha'
    $deltaCount = $null
    if ($null -ne $completedSha -and $null -ne $upstreamSha) {
        if ($completedSha -eq $upstreamSha) {
            $deltaCount = 0
        }
        elseif (Test-Ancestor -Ancestor $completedSha -Descendant $upstreamSha) {
            $deltaCount = Get-CommitCount -Range "$completedSha..$upstreamSha"
        }
        else {
            $deltaCount = -1
        }
    }

    $missingReviews = @()
    if ($null -ne $pending) {
        $reviews = Get-PropertyValue -Object $pending -Name 'reviews' -Default (New-EmptyObject)
        foreach ($id in (Get-ActiveCustomizationIds)) {
            $review = Get-ReviewValue -Reviews $reviews -Id $id
            if ($null -eq $review -or (Get-PropertyValue -Object $review -Name 'status' -Default '') -ne 'reviewed') {
                $missingReviews += $id
            }
        }
    }

    $status = 'READY'
    $exitCode = 0
    $message = 'Recorded synchronization state is current.'
    if ($unrelated.Count -gt 0) {
        $status = 'DIRTY'
        $exitCode = 10
        $message = 'Unrelated working-tree changes require attention.'
    }
    elseif ($null -eq $upstreamSha -or $null -eq $mainSha -or $null -eq $customNextSha) {
        $status = 'BLOCKED'
        $exitCode = 20
        $message = 'Required Git references are missing.'
    }
    elseif ($null -ne $pending -and $pendingSha -ne $upstreamSha) {
        $status = 'STALE_PENDING_REVIEW'
        $exitCode = 10
        $message = 'Pending review targets a different upstream SHA.'
    }
    elseif ($null -ne $pending) {
        $status = 'PENDING_REVIEW'
        $exitCode = 10
        $message = 'A synchronization transaction is in progress.'
    }
    elseif ($null -eq $completed) {
        $status = 'UNINITIALIZED'
        $exitCode = 10
        $message = 'No completed synchronization checkpoint exists.'
    }
    elseif ($null -eq $completedSha -or $completedSha -ne $upstreamSha) {
        $status = 'OUT_OF_DATE'
        $exitCode = 10
        $message = 'upstream/main has advanced beyond the last completed checkpoint.'
    }

    return New-Result -CommandName 'status' -Status $status -ExitCode $exitCode -Message $message -Data ([pscustomobject]@{
        current_branch = $branch
        working_tree_changes = @($changes)
        unrelated_worktree_changes = @($unrelated)
        upstream_main_sha = $upstreamSha
        main_sha = $mainSha
        custom_next_sha = $customNextSha
        recorded_upstream_baseline_sha = $completedSha
        upstream_commits_not_recorded = $deltaCount
        pending_candidate_upstream_sha = $pendingSha
        pending_missing_reviews = $missingReviews
        state_path = $script:StateRelativePath
    })
}

function Invoke-Bootstrap {
    Assert-DevelopmentBranch
    Assert-NoUnrelatedWorktreeChanges
    if (Test-Path -LiteralPath $script:StatePath -PathType Leaf) {
        throw "State file already exists. Bootstrap is only for first initialization."
    }

    $upstreamSha = Get-RequiredRefSha -Reference 'upstream/main'
    $mainSha = Get-RequiredRefSha -Reference 'main'
    $customNextSha = Get-RequiredRefSha -Reference 'custom-next'
    if (-not (Test-Ancestor -Ancestor $upstreamSha -Descendant $mainSha)) {
        throw "main does not contain upstream/main. Update main before bootstrap."
    }
    if (-not (Test-Ancestor -Ancestor $mainSha -Descendant $customNextSha)) {
        throw "custom-next does not contain main. Bootstrap requires the new baseline."
    }

    $state = New-InitialState -CandidateUpstreamSha $upstreamSha -CustomNextSha $customNextSha
    Save-State -State $state
    return New-Result -CommandName 'bootstrap' -Status 'PENDING_REVIEW' -Message 'Bootstrap pending created. Review active customizations, then run record.' -Data ([pscustomobject]@{
        candidate_upstream_baseline_sha = $upstreamSha
        main_integration_commit = $mainSha
        custom_next_sync_commit = (Get-SyncPosition -MainSha $mainSha -CustomNextSha $customNextSha)
        custom_next_sync_kind = 'base'
        active_customizations = @(Get-ActiveCustomizationIds)
        state_path = $script:StateRelativePath
    })
}

function Invoke-Prepare {
    Assert-DevelopmentBranch
    Assert-NoUnrelatedWorktreeChanges
    $state = Load-State
    if ($null -eq $state) {
        throw "State file does not exist. Run bootstrap before prepare."
    }

    [void](Invoke-Git -Arguments @('fetch', 'upstream', 'main'))
    $candidateSha = Get-RequiredRefSha -Reference 'upstream/main'
    $pending = Get-PropertyValue -Object $state -Name 'pending_review'
    if ($null -ne $pending) {
        $pendingSha = Get-PropertyValue -Object $pending -Name 'candidate_upstream_baseline_sha'
        if ($pendingSha -ne $candidateSha) {
            return New-Result -CommandName 'prepare' -Status 'STALE_PENDING_REVIEW' -ExitCode 20 -Message 'An existing pending review targets a different upstream SHA. Complete it or run abandon.' -Data ([pscustomobject]@{
                existing_pending_sha = $pendingSha
                current_upstream_sha = $candidateSha
            })
        }

        return New-Result -CommandName 'prepare' -Status 'PENDING_REVIEW' -ExitCode 10 -Message 'Existing pending review retained; no state was reset.' -Data ([pscustomobject]@{
            candidate_upstream_baseline_sha = $candidateSha
            pending_reviews = @((Get-PropertyValue -Object $pending -Name 'reviews' -Default (New-EmptyObject)).PSObject.Properties.Name)
        })
    }

    $completed = Get-PropertyValue -Object $state -Name 'last_completed_sync'
    $previousSha = Get-PropertyValue -Object $completed -Name 'upstream_baseline_sha'
    if ($null -eq $previousSha) {
        throw "No completed baseline exists. Run bootstrap instead of prepare."
    }
    if ($previousSha -eq $candidateSha) {
        return New-Result -CommandName 'prepare' -Status 'READY' -Message 'upstream/main has not advanced; no pending review was created.' -Data ([pscustomobject]@{
            upstream_baseline_sha = $candidateSha
            changed_files = @()
        })
    }
    if (-not (Test-Ancestor -Ancestor $previousSha -Descendant $candidateSha)) {
        throw "The candidate upstream SHA is not a descendant of the recorded baseline. Manual history inspection is required."
    }

    $customNextSha = Get-RequiredRefSha -Reference 'custom-next'
    $pending = New-PendingReview `
        -CandidateUpstreamSha $candidateSha `
        -PreviousUpstreamSha $previousSha `
        -PreparedFromCustomNextCommit $customNextSha
    Set-PropertyValue -Object $state -Name 'pending_review' -Value $pending
    Save-State -State $state

    return New-Result -CommandName 'prepare' -Status 'PENDING_REVIEW' -ExitCode 10 -Message 'Pending review created. Synchronize branches, review customizations, validate, then run record.' -Data ([pscustomobject]@{
        previous_upstream_baseline_sha = $previousSha
        candidate_upstream_baseline_sha = $candidateSha
        upstream_commit_count = (Get-CommitCount -Range "$previousSha..$candidateSha")
        changed_files = @(Get-ChangedFiles -FromSha $previousSha -ToSha $candidateSha)
        active_customizations = @(Get-ActiveCustomizationIds)
    })
}

function Invoke-Review {
    Assert-DevelopmentBranch
    Assert-NoUnrelatedWorktreeChanges
    if ([string]::IsNullOrWhiteSpace($Id)) {
        Stop-InvalidInput -Message 'review requires a customization ID.'
    }
    if ([string]::IsNullOrWhiteSpace($Impact) -or @('unaffected', 'adapted', 'upstream-equivalent', 'unknown') -notcontains $Impact) {
        Stop-InvalidInput -Message 'review requires -Impact with one of: unaffected, adapted, upstream-equivalent, unknown.'
    }

    $activeIds = Get-ActiveCustomizationIds
    if ($activeIds -notcontains $Id) {
        Stop-InvalidInput -Message "Customization '$Id' is not an active entry in customization-map.md."
    }
    if (($Impact -eq 'adapted' -or $Impact -eq 'upstream-equivalent') -and [string]::IsNullOrWhiteSpace($Note)) {
        # The note is optional; callers can add it when the semantic decision needs context.
    }

    $state = Load-State
    if ($null -eq $state) {
        throw 'State file does not exist. Run bootstrap first.'
    }
    $pending = Get-PropertyValue -Object $state -Name 'pending_review'
    if ($null -eq $pending) {
        throw 'No pending review exists. Run prepare or bootstrap first.'
    }

    $candidateSha = Get-PropertyValue -Object $pending -Name 'candidate_upstream_baseline_sha'
    $upstreamSha = Get-RequiredRefSha -Reference 'upstream/main'
    if ($candidateSha -ne $upstreamSha) {
        throw 'Pending review is stale because upstream/main has changed. Run status and resolve the pending transaction.'
    }

    $mainSha = Get-RequiredRefSha -Reference 'main'
    $customNextSha = Get-RequiredRefSha -Reference 'custom-next'
    if (-not (Test-Ancestor -Ancestor $candidateSha -Descendant $mainSha)) {
        throw 'main does not contain the pending upstream candidate.'
    }
    if (-not (Test-Ancestor -Ancestor $mainSha -Descendant $customNextSha)) {
        throw 'custom-next does not contain main. Complete branch synchronization before review.'
    }

    if ((Get-PropertyValue -Object $pending -Name 'status' -Default '') -eq 'bootstrap') {
        $syncCommit = $mainSha
        $syncKind = 'base'
    }
    else {
        $syncCommit = Get-SyncPosition -MainSha $mainSha -CustomNextSha $customNextSha
        $syncKind = Get-SyncKind -MainSha $mainSha -CustomNextSha $customNextSha
    }
    if ($syncKind -eq 'unknown') {
        Stop-Blocked -Message 'Unable to determine custom-next synchronization kind without guessing.'
    }

    $reviews = Get-PropertyValue -Object $pending -Name 'reviews' -Default (New-EmptyObject)
    $reviewStatus = if ($Impact -eq 'unknown') { 'needs-review' } else { 'reviewed' }
    $review = [pscustomobject]@{
        status = $reviewStatus
        impact = $Impact
        note = if ([string]::IsNullOrWhiteSpace($Note)) { $null } else { $Note }
        last_reviewed_upstream_sha = $candidateSha
        last_reviewed_custom_next_commit = $syncCommit
        reviewed_at = Get-UtcTimestamp
    }
    Set-ReviewValue -Reviews $reviews -Id $Id -Value $review
    Set-PropertyValue -Object $pending -Name 'reviews' -Value $reviews
    Set-PropertyValue -Object $pending -Name 'main_integration_commit' -Value $mainSha
    Set-PropertyValue -Object $pending -Name 'custom_next_sync_commit' -Value $syncCommit
    Set-PropertyValue -Object $pending -Name 'custom_next_sync_kind' -Value $syncKind
    Set-PropertyValue -Object $state -Name 'pending_review' -Value $pending
    Save-State -State $state

    return New-Result -CommandName 'review' -Status $reviewStatus -Message "Recorded review for $Id." -Data $review
}

function Invoke-Record {
    Assert-DevelopmentBranch
    Assert-NoUnrelatedWorktreeChanges
    if ([string]::IsNullOrWhiteSpace($ValidationStatus) -or $ValidationStatus -ne 'passed') {
        Stop-InvalidInput -Message "record requires -ValidationStatus passed. Received '$ValidationStatus'."
    }

    $state = Load-State
    if ($null -eq $state) {
        throw 'State file does not exist. Run bootstrap first.'
    }
    $pending = Get-PropertyValue -Object $state -Name 'pending_review'
    if ($null -eq $pending) {
        throw 'No pending review exists. Run prepare or bootstrap first.'
    }

    $candidateSha = Get-PropertyValue -Object $pending -Name 'candidate_upstream_baseline_sha'
    $upstreamSha = Get-RequiredRefSha -Reference 'upstream/main'
    $mainSha = Get-RequiredRefSha -Reference 'main'
    $customNextSha = Get-RequiredRefSha -Reference 'custom-next'
    if ($candidateSha -ne $upstreamSha) {
        Stop-Blocked -Message 'Pending review is stale because upstream/main has changed.'
    }
    if (-not (Test-Ancestor -Ancestor $candidateSha -Descendant $mainSha)) {
        Stop-Blocked -Message 'main does not contain the pending upstream candidate.'
    }
    if (-not (Test-Ancestor -Ancestor $mainSha -Descendant $customNextSha)) {
        Stop-Blocked -Message 'custom-next does not contain main.'
    }

    $reviews = Get-PropertyValue -Object $pending -Name 'reviews' -Default (New-EmptyObject)
    $missing = New-Object System.Collections.Generic.List[string]
    foreach ($id in (Get-ActiveCustomizationIds)) {
        $review = Get-ReviewValue -Reviews $reviews -Id $id
        $reviewStatus = Get-PropertyValue -Object $review -Name 'status' -Default ''
        $reviewImpact = Get-PropertyValue -Object $review -Name 'impact' -Default 'unknown'
        if ($reviewStatus -ne 'reviewed' -or $reviewImpact -eq 'unknown') {
            $missing.Add($id)
        }
        if ((Get-PropertyValue -Object $review -Name 'last_reviewed_upstream_sha' -Default '') -ne $candidateSha) {
            if (-not $missing.Contains($id)) {
                $missing.Add($id)
            }
        }
    }
    if ($missing.Count -gt 0) {
        Stop-Blocked -Message "Cannot record synchronization; customizations require review: $($missing -join ', ')"
    }

    $validation = Get-PropertyValue -Object $pending -Name 'validation' -Default (New-EmptyObject)
    $validationStatus = Get-PropertyValue -Object $validation -Name 'status' -Default 'not-run'
    if ($validationStatus -in @('failed', 'blocked')) {
        Stop-Blocked -Message "Pending validation status is '$validationStatus'; a new validation result is required."
    }
    Set-PropertyValue -Object $pending -Name 'validation' -Value ([pscustomobject]@{
        status = 'passed'
        note = if ([string]::IsNullOrWhiteSpace($ValidationNote)) { $null } else { $ValidationNote }
        validated_at = Get-UtcTimestamp
    })

    $syncCommit = Get-SyncPosition -MainSha $mainSha -CustomNextSha $customNextSha
    $syncKind = Get-SyncKind -MainSha $mainSha -CustomNextSha $customNextSha
    if ((Get-PropertyValue -Object $pending -Name 'status' -Default '') -eq 'bootstrap') {
        $syncCommit = $mainSha
        $syncKind = 'base'
    }
    if ($syncKind -eq 'unknown') {
        Stop-Blocked -Message 'Unable to determine custom-next synchronization kind without guessing.'
    }

    $completed = [pscustomobject]@{
        status = 'complete'
        upstream_baseline_sha = $candidateSha
        main_integration_commit = $mainSha
        custom_next_sync_commit = $syncCommit
        custom_next_sync_kind = $syncKind
        validation_status = 'passed'
        validation_note = if ([string]::IsNullOrWhiteSpace($ValidationNote)) { $null } else { $ValidationNote }
        recorded_at = Get-UtcTimestamp
    }
    Set-PropertyValue -Object $state -Name 'last_completed_sync' -Value $completed
    Set-PropertyValue -Object $state -Name 'customization_reviews' -Value $reviews
    Set-PropertyValue -Object $state -Name 'pending_review' -Value $null
    Save-State -State $state

    return New-Result -CommandName 'record' -Status 'READY' -Message 'Completed synchronization checkpoint recorded.' -Data $completed
}

function Invoke-Abandon {
    Assert-DevelopmentBranch
    Assert-NoUnrelatedWorktreeChanges
    if ([string]::IsNullOrWhiteSpace($Reason)) {
        Stop-InvalidInput -Message 'abandon requires a non-empty -Reason.'
    }

    $state = Load-State
    if ($null -eq $state) {
        throw 'State file does not exist.'
    }
    $pending = Get-PropertyValue -Object $state -Name 'pending_review'
    if ($null -eq $pending) {
        throw 'No pending review exists to abandon.'
    }

    $abandoned = [pscustomobject]@{
        candidate_upstream_baseline_sha = Get-PropertyValue -Object $pending -Name 'candidate_upstream_baseline_sha'
        reason = $Reason
        abandoned_at = Get-UtcTimestamp
    }
    Set-PropertyValue -Object $state -Name 'pending_review' -Value $null
    Save-State -State $state

    return New-Result -CommandName 'abandon' -Status 'READY' -Message 'Pending review abandoned; last completed checkpoint was preserved.' -Data $abandoned
}

function Write-HumanResult {
    param(
        [Parameter(Mandatory = $true)]
        [object]$Result
    )

    Write-Output "Fork Sync $($Result.command)"
    Write-Output ('-' * 32)
    Write-Output "Status: $($Result.status)"
    if (-not [string]::IsNullOrWhiteSpace($Result.message)) {
        Write-Output $Result.message
    }
    if ($null -ne $Result.data) {
        foreach ($property in $Result.data.PSObject.Properties) {
            $value = $property.Value
            if ($value -is [System.Collections.IEnumerable] -and -not ($value -is [string])) {
                $display = (($value | ForEach-Object { $_.ToString() }) -join ', ')
            }
            elseif ($null -eq $value) {
                $display = '<none>'
            }
            else {
                $display = $value.ToString()
            }
            Write-Output ("{0}: {1}" -f $property.Name, $display)
        }
    }
}

function Write-ResultAndExit {
    param(
        [Parameter(Mandatory = $true)]
        [object]$Result
    )

    if ($script:JsonOutput) {
        $Result | ConvertTo-Json -Depth 20
    }
    else {
        Write-HumanResult -Result $Result
    }
    exit ([int]$Result.exit_code)
}

try {
    Assert-CommandAllowed
    switch ($Command) {
        'status' { $result = Get-StatusResult }
        'prepare' { $result = Invoke-Prepare }
        'review' { $result = Invoke-Review }
        'record' { $result = Invoke-Record }
        'bootstrap' { $result = Invoke-Bootstrap }
        'abandon' { $result = Invoke-Abandon }
    }
    Write-ResultAndExit -Result $result
}
catch {
    $message = $_.Exception.Message
    $exitCode = 2
    $status = 'ERROR'
    if ($message.StartsWith('BLOCKED:')) {
        $exitCode = 20
        $status = 'BLOCKED'
        $message = $message.Substring('BLOCKED:'.Length).Trim()
    }
    elseif ($message.StartsWith('INVALID_INPUT:')) {
        $message = $message.Substring('INVALID_INPUT:'.Length).Trim()
    }
    $errorResult = New-Result -CommandName $Command -Status $status -ExitCode $exitCode -Message $message
    if ($script:JsonOutput) {
        $errorResult | ConvertTo-Json -Depth 20
    }
    else {
        Write-Error $message
    }
    exit $exitCode
}
