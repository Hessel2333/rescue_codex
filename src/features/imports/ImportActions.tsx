import { FolderOpen, ScanSearch } from "lucide-react";
import { Button } from "../../components/Button";

type ImportActionsProps = {
  busy: boolean;
  onScanDefault: () => void;
  onImportFiles: () => void;
  onImportFolders: () => void;
};

export function ImportActions({
  busy,
  onScanDefault,
  onImportFiles,
  onImportFolders,
}: ImportActionsProps) {
  return (
    <div className="flex flex-wrap gap-3">
      <Button disabled={busy} onClick={onScanDefault} icon={<ScanSearch className="h-4 w-4" />}>
        扫描默认 Codex 目录
      </Button>
      <Button disabled={busy} variant="secondary" onClick={onImportFiles} icon={<FolderOpen className="h-4 w-4" />}>
        导入文件
      </Button>
      <Button disabled={busy} variant="secondary" onClick={onImportFolders} icon={<FolderOpen className="h-4 w-4" />}>
        导入文件夹
      </Button>
    </div>
  );
}
