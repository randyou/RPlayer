param([string]$File)
$j = Get-Content $File -Raw | ConvertFrom-Json
function Walk($x) {
    if ($x.states) {
        $s = ($x.states -join ',')
        if ($s -match 'focused') { return "$($x.role) | $($x.name)" }
    }
    foreach ($c in $x.children) {
        $r = Walk $c
        if ($r) { return $r }
    }
    return $null
}
Walk $j.data.tree
