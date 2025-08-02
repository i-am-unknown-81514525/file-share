import marimo

__generated_with = "0.14.16"
app = marimo.App(width="medium")


@app.cell
def _():
    import requests
    import marimo as mo
    return mo, requests


@app.cell
def _(mo):
    instance_field = mo.ui.text(label="Instance ID")

    instance_field
    return (instance_field,)


@app.cell
def _(mo):
    file_upload = mo.ui.file(label="File Upload", max_size=512*1024)
    upload_button = mo.ui.run_button(label="Upload")

    mo.hstack(
        [
            file_upload, upload_button
        ]
    )
    return file_upload, upload_button


@app.cell
def _(file_upload, instance_field, mo, requests, upload_button):
    mo.stop(not upload_button.value)
    mo.stop(file_upload.contents() is None)
    upload_output = requests.post(
        f"https://file-share.iamaunknownpeople.workers.dev/upload/{instance_field.value}",
        data=file_upload.contents()
    )
    upload_status = upload_output.status_code
    upload_id = upload_output.text

    mo.md(f"Status: `{upload_status}`\n\nID: `{upload_id}`")
    return


@app.cell
def _(mo):
    file_download = mo.ui.text(label="File ID")
    filename = mo.ui.text(label="Filename")
    download_button = mo.ui.run_button(label="Download to memory")

    mo.hstack(
        [
            mo.vstack([file_download, filename]), download_button
        ]
    )
    return download_button, file_download, filename


@app.cell
def _(download_button, file_download, filename, instance_field, mo, requests):
    mo.stop(not download_button.value)
    mo.stop(not file_download.value)
    download_output = requests.get(f"https://file-share.iamaunknownpeople.workers.dev/download/{instance_field.value}:::{file_download.value}")
    download_status = download_output.status_code
    actual_download_button = mo.download(data=download_output.content, label="Download to device", filename=filename.value)

    mo.md(f"Status: `{download_status}`\n\nFile: {actual_download_button}" if download_status < 400 else f"Status: `{download_status}`")
    return


if __name__ == "__main__":
    app.run()
