param([int]$Count=1,[string]$Out="D:\Workspace\_x")
function Find-Node($n,[string]$name){ if($n.name -eq $name){return $n}; foreach($c in $n.children){ $r=Find-Node $c $name; if($r){return $r} }; return $null }
for($i=1;$i -le $Count;$i++){
  agent-desktop snapshot --window-id w-28314196 > _s.json 2>&1
  $j=Get-Content _s.json -Raw | ConvertFrom-Json
  $node=Find-Node $j.data.tree '下一首'
  if(-not $node){ "no next button"; break }
  $res = agent-desktop click $node.ref_id 2>&1 | ConvertFrom-Json
  "click $i -> $($res.ok)"
  Start-Sleep -Milliseconds 1400
  & D:\Workspace\_cap.ps1 -Out "$Out$i"
}
