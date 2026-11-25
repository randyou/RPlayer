param([string]$File,[string]$Mode = "ref")
$j = Get-Content $File -Raw | ConvertFrom-Json
function Walk($n) {
    if ($n.role -eq 'switch' -and $n.name -match '顺序|循环|单曲|随机') {
        if ($Mode -eq 'ref') { return $n.ref_id } else { return $n.name }
    }
    foreach ($c in $n.children) {
        $r = Walk $c
        if ($r) { return $r }
    }
    return $null
}
Walk $j.data.tree
