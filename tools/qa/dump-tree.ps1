function Show-Nodes($n,[int]$d){ "{0}{1} | {2} | {3}" -f (' ' * $d), $n.role, $n.name, $n.ref_id; foreach($c in $n.children){ Show-Nodes $c ($d+1) } }
$j=Get-Content D:\Workspace\_ov.json -Raw | ConvertFrom-Json
Show-Nodes $j.data.tree 0
