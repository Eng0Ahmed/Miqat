' تشغيل برنامج ميقات بشكل صامت تماماً بدون ظهور أي نافذة سوداء
Set WshShell = CreateObject("WScript.Shell")
Dim scriptDir
scriptDir = CreateObject("Scripting.FileSystemObject").GetParentFolderName(WScript.ScriptFullName)
WshShell.CurrentDirectory = scriptDir
WshShell.Run """" & scriptDir & "\ميقات.exe""", 0, False
