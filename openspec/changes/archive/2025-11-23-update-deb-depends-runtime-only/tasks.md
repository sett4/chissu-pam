## 1. Spec Update
- [x] 1.1 Update packaging-deb spec to remove -dev from runtime Depends and note Build-Depends placement.

## 2. Implementation
- [x] 2.1 Update build/package/debian/control.in Build-Depends to include required -dev libs.
- [x] 2.2 Update Depends to rely on ${shlibs:Depends} + misc and non-lib tools only.

## 3. Validation
- [ ] 3.1 (Optional) Run dpkg-deb -I on generated package to confirm Depends shrink (manual after build).
