param([string]$File)
$j = Get-Content $File -Raw | ConvertFrom-Json
$total = 0
$unnamed = @()
function Walk($n) {
    $interactive = @('switch', 'button', 'checkbox', 'textfield', 'slider', 'combobox', 'radiobutton', 'menuitem')
    if ($interactive -contains $n.role) {
        $script:total++
        if (-not $n.name -or $n.name.Trim() -eq '') {
            $script:unnamed += "$($n.role) | $($n.ref_id)"
        }
    }
    foreach ($c in $n.children) { Walk $c }
}
Walk $j.data.tree
"interactive=$total unnamed=$($unnamed.Count)"
$unnamed | Select-Object -First 12
